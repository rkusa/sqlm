#![cfg_attr(nightly_column_names, feature(adt_const_params))]
#![cfg_attr(nightly_column_names, allow(incomplete_features))]

// Necessary to have `::sqlm_postgres::` available in tests
#[cfg(test)]
extern crate self as sqlm_postgres;

mod error;
mod future;
mod row;

use std::env;
use std::marker::PhantomData;
use std::str::FromStr;

use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod};
pub use error::Error;
pub use future::SqlFuture;
use once_cell::sync::OnceCell;
pub use row::{AnyCols, FromRow, HasColumn, Row};
use tokio_postgres::config::SslMode;
use tokio_postgres::types::ToSql;
use tokio_postgres::NoTls;

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
                        .with_root_certificates(rustls::RootCertStore::empty())
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

pub struct Sql<'a, Cols, T = ()> {
    // TODO: not pub?
    pub query: &'static str,
    pub parameters: &'a [&'a (dyn ToSql + Sync)],
    pub marker: PhantomData<(Cols, T)>,
}

#[cfg(test)]
mod tests {
    use sqlm_postgres_macros::sql;

    use crate::row::{FromRow, Row};
    use crate::HasColumn;

    #[tokio::test]
    async fn from_row() {
        #[derive(Debug, PartialEq, Eq)]
        struct User {
            id: i64,
            name: String,
        }

        impl<Cols> FromRow<Cols> for User
        where
            Cols: HasColumn<i64, "id">,
            Cols: HasColumn<String, "name">,
            // Cols: HasColumn<String, 3546873949167855552>,
            // Cols: HasColumn<i64, 6898215271518772730>,
        {
            fn from_row(row: Row<Cols>) -> Result<Self, tokio_postgres::Error> {
                Ok(User {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                })
            }
        }

        // impl FromRow<AnyCols> for User {
        //     fn from_row(row: Row<AnyCols>) -> Result<Self, tokio_postgres::Error> {
        //         Ok(User {
        //             id: row.try_get("id")?,
        //             name: row.try_get("name")?,
        //         })
        //     }
        // }

        let id = 1;
        let user: User = sql!("SELECT id, name FROM users WHERE id = {id}")
            .await
            .unwrap();
        assert_eq!(
            user,
            User {
                id: 1,
                name: "first".to_string()
            }
        );
    }
}
