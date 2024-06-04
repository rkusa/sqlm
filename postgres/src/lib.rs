#![cfg_attr(nightly_column_names, feature(adt_const_params))]
#![cfg_attr(nightly_column_names, allow(incomplete_features))]

// Necessary to have `::sqlm_postgres::` available in tests
#[cfg(test)]
extern crate self as sqlm_postgres;

mod connection;
mod error;
mod future;
pub mod internal;
mod query;
mod row;
pub mod types;

use std::env;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

pub use connection::Connection;
pub use deadpool_postgres::{Client, Transaction};
use deadpool_postgres::{ClientWrapper, Manager, ManagerConfig, Pool, RecyclingMethod};
pub use error::Error;
use error::ErrorKind;
pub use future::SqlFuture;
use once_cell::sync::OnceCell;
pub use query::Query;
pub use row::{FromRow, Row};
pub use sqlm_postgres_macros::{sql, Enum, FromRow};
pub use tokio_postgres;
use tokio_postgres::config::SslMode;
use tokio_postgres::types::ToSql;
use tokio_postgres::NoTls;
pub use types::SqlType;

static POOL: OnceCell<Pool> = OnceCell::new();

#[tracing::instrument]
pub async fn connect() -> Result<deadpool_postgres::Client, Error> {
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

pub struct Sql<'a, Cols, T> {
    // TODO: not pub?
    pub query: &'static str,
    pub parameters: &'a [&'a (dyn ToSql + Sync)],
    pub transaction: Option<&'a Transaction<'a>>,
    pub connection: Option<&'a ClientWrapper>,
    pub marker: PhantomData<(Cols, T)>,
}

impl<'a, Cols, T> Sql<'a, Cols, T> {
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
