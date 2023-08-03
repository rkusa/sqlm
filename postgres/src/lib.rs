#![cfg_attr(nightly_column_names, feature(adt_const_params))]
#![cfg_attr(nightly_column_names, allow(incomplete_features))]

// Necessary to have `::sqlm_postgres::` available in tests
#[cfg(test)]
extern crate self as sqlm_postgres;

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

pub use deadpool_postgres::Transaction;
use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod};
pub use error::Error;
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

pub async fn connect() -> Result<Object, Error> {
    // Don't trace connect, as this would create an endless loop of connecting again and
    // again when persisting the connect trace!
    let pool = POOL.get_or_try_init(|| {
        let mut config = tokio_postgres::Config::from_str(
            dotenvy::var("DATABASE_URL")
                .map_err(|_| Error::MissingDatabaseUrlEnv)?
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
                        .with_safe_defaults()
                        .with_custom_certificate_verifier(Arc::new(NoServerCertVerify))
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
    pub marker: PhantomData<(Cols, T)>,
}

impl<'a, Cols, T> Sql<'a, Cols, T> {
    pub fn with_transaction(mut self, tx: &'a Transaction<'a>) -> Self {
        self.transaction = Some(tx);
        self
    }

    async fn query_one(&self) -> Result<tokio_postgres::Row, Error> {
        if let Some(tx) = self.transaction {
            let stmt = tx.prepare_cached(self.query).await?;
            Ok(tx.query_one(&stmt, self.parameters).await?)
        } else {
            let conn = connect().await?;
            let stmt = conn.prepare_cached(self.query).await?;
            Ok(conn.query_one(&stmt, self.parameters).await?)
        }
    }

    async fn query_opt(&self) -> Result<Option<tokio_postgres::Row>, Error> {
        if let Some(tx) = self.transaction {
            let stmt = tx.prepare_cached(self.query).await?;
            Ok(tx.query_opt(&stmt, self.parameters).await?)
        } else {
            let conn = connect().await?;
            let stmt = conn.prepare_cached(self.query).await?;
            Ok(conn.query_opt(&stmt, self.parameters).await?)
        }
    }

    async fn query(&self) -> Result<Vec<tokio_postgres::Row>, Error> {
        if let Some(tx) = self.transaction {
            let stmt = tx.prepare_cached(self.query).await?;
            Ok(tx.query(&stmt, self.parameters).await?)
        } else {
            let conn = connect().await?;
            let stmt = conn.prepare_cached(self.query).await?;
            Ok(conn.query(&stmt, self.parameters).await?)
        }
    }

    async fn execute(&self) -> Result<(), Error> {
        if let Some(tx) = self.transaction {
            let stmt = tx.prepare_cached(self.query).await?;
            tx.execute(&stmt, self.parameters).await?;
        } else {
            let conn = connect().await?;
            let stmt = conn.prepare_cached(self.query).await?;
            conn.execute(&stmt, self.parameters).await?;
        }
        Ok(())
    }
}

struct NoServerCertVerify;

impl rustls::client::ServerCertVerifier for NoServerCertVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp: &[u8],
        _now: std::time::SystemTime,
    ) -> std::result::Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
