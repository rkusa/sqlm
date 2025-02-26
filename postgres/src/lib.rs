#![cfg_attr(nightly_column_names, feature(adt_const_params, unsized_const_params))]
#![cfg_attr(nightly_column_names, allow(incomplete_features))]
#![forbid(unsafe_code)]

//! An [`sql!`] macro to write compile-time checked database queries similar to how [`format!`]
//! works.
//!
//! # Example
//!
//! ```
//! use sqlm_postgres::{sql, Enum, FromRow, FromSql, ToSql};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let id: i64 = 1;
//! let user: User = sql!("SELECT * FROM users WHERE id = {id}").await?;
//!
//! #[derive(Debug, FromRow)]
//! struct User {
//!     id: i64,
//!     name: String,
//!     role: Role,
//! }
//!
//! #[derive(Debug, Default, FromSql, ToSql, Enum)]
//! #[postgres(name = "role")]
//! enum Role {
//!     #[default]
//!     #[postgres(name = "user")]
//!     User,
//!     #[postgres(name = "admin")]
//!     Admin,
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Usage
//! - Add `sqlm-postgres` to your dependencies
//! - Make the `DATABASE_URL` env variable available during compile time (e.g. via adding an `.env`
//!   file)
//! - Start using the [`sql!`] macro (no further setup necessary; a connection pool is automatically
//!   created for you)
//!
//! # Caveats
//! - Automatically creates a global connection pool for you with no way to opt out
//! - Compile-time checks cannot be disabled. Thus also requires database access on your CI.
//! - Does not know whether rows returned from Postgres are nullable and consequentially
//!   requires all types to implement [`Default::default`], which it falls back to if Postgres
//!   returns null.

// Necessary to have `::sqlm_postgres::` available in tests
#[cfg(test)]
extern crate self as sqlm_postgres;

mod connection;
pub mod error;
mod future;
#[doc(hidden)]
pub mod internal;
mod macros;
mod query;
mod row;
#[doc(hidden)]
pub mod types;

use std::env;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

pub use connection::Connection;
use deadpool_postgres::{ClientWrapper, Manager, ManagerConfig, Pool, RecyclingMethod};
pub use error::Error;
use error::ErrorKind;
pub use future::SqlFuture;
pub use macros::{Enum, FromRow, sql};
use once_cell::sync::OnceCell;
use query::Query;
pub use row::{FromRow, Row};
pub use tokio_postgres;
use tokio_postgres::NoTls;
use tokio_postgres::config::SslMode;
pub use tokio_postgres::types::{FromSql, ToSql};
pub use types::SqlType;

static POOL: OnceCell<Pool> = OnceCell::new();

/// A database transaction.
pub type Transaction<'t> = deadpool_postgres::Transaction<'t>;

/// An asynchronous PostgreSQL client (basically a non-transactional connection).
pub type Client = deadpool_postgres::Client;

/// Establish a database connection.
///
/// This function is automatically called when awaiting queries created with [`sql!`]. When first
/// called, a connection pool is created, which expects the env variable `DATABASE_URL` to be set.
///
/// When having multiple sequential queries, it is recommended to manually establish a connection
/// and pass it to [`sql!`] via [`Sql::run_with`].
///
/// # Examples
///
/// ```
/// # use sqlm_postgres::{sql, connect, Enum, FromRow, FromSql, ToSql};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let conn = connect().await?;
/// let name: String = sql!("SELECT name FROM users WHERE id = {id}", id = 1i64)
///     .run_with(conn)
///     .await?;
/// # Ok(())
/// # }
/// ```
/// ```
/// # use sqlm_postgres::{sql, connect, Enum, FromRow, FromSql, ToSql};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut conn = connect().await?;
/// let tx = conn.transaction().await?;
/// let name: String = sql!("SELECT name FROM users WHERE id = {id}", id = 1i64)
///     .run_with(&tx)
///     .await?;
/// tx.commit().await?;
/// # Ok(())
/// # }
/// ```
#[tracing::instrument]
pub async fn connect() -> Result<Client, Error> {
    // Don't trace connect, as this would create an endless loop of connecting again and
    // again when persisting the connect trace!
    let pool = POOL.get_or_try_init(|| {
        let mut config = tokio_postgres::Config::from_str(
            dotenvy::var("DATABASE_URL")
                .map_err(|_| ErrorKind::MissingDatabaseUrlEnv)?
                .as_str(),
        )?;
        config.application_name(env!("CARGO_PKG_NAME"));

        // TODO: take all possible SSL variants into account, see e.g.
        // https://github.com/jbg/tokio-postgres-rustls/issues/11
        let mgr = match config.get_ssl_mode() {
            SslMode::Disable => Manager::from_config(
                config,
                NoTls,
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
            ),
            _ => Manager::from_config(
                config,
                {
                    let config = rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(Arc::new(NoServerCertVerify::default()))
                        .with_no_client_auth();
                    tokio_postgres_rustls::MakeRustlsConnect::new(config)
                },
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
            ),
        };
        let pool = Pool::builder(mgr).max_size(4).build()?;
        Ok::<_, Error>(pool)
    })?;
    let conn = pool.get().await?;
    Ok(conn)
}

/// The struct created by [`sql!`]; executed by calling `.await`.
pub struct Sql<'a, Cols, T> {
    // Fields need to be public so that they can be set by the macro invocation.
    #[doc(hidden)]
    pub query: &'static str,
    #[doc(hidden)]
    pub parameters: &'a [&'a (dyn ToSql + Sync)],
    #[doc(hidden)]
    pub transaction: Option<&'a Transaction<'a>>,
    #[doc(hidden)]
    pub connection: Option<&'a ClientWrapper>,
    #[doc(hidden)]
    pub marker: PhantomData<(Cols, T)>,
}

impl<'a, Cols, T> Sql<'a, Cols, T> {
    /// Manually pass a connection or transaction to a query created with [`sql!`].
    ///
    /// See [`connect`] for examples.
    pub fn run_with(self, conn: impl Connection + 'a) -> SqlFuture<'a, T>
    where
        T: Query<Cols> + Send + Sync + 'a,
        Cols: Send + Sync + 'a,
    {
        SqlFuture::with_connection(self, conn)
    }
}

#[derive(Debug)]
struct NoServerCertVerify {
    crypto_provider: Arc<rustls::crypto::CryptoProvider>,
}

impl Default for NoServerCertVerify {
    fn default() -> Self {
        Self {
            crypto_provider: Arc::clone(
                rustls::crypto::CryptoProvider::get_default()
                    .expect("no default provider for rustls installed"),
            ),
        }
    }
}

impl rustls::client::danger::ServerCertVerifier for NoServerCertVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.crypto_provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
