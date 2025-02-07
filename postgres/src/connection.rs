// TODO: remove once Rust's async lifetime in trait story got improved
#![allow(clippy::manual_async_fn)]

use std::future::Future;

use deadpool_postgres::GenericClient;
use tokio_postgres::types::ToSql;
use tokio_postgres::Row;

use crate::error::ErrorKind;
use crate::Error;

/// A trait used to allow functions to accept connections without having to explicit about whether
/// it's a transaction or not.
///
/// # Example
/// ```
/// # use sqlm_postgres::{sql, Connection};
/// pub async fn fetch_username(id: i64, conn: impl Connection) -> Result<String, sqlm_postgres::Error> {
///     sql!("SELECT name FROM users WHERE id = {id}")
///         .run_with(conn)
///         .await
/// }
/// ```
pub trait Connection: Send + Sync {
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a;

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a;

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a;

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a;
}

impl Connection for deadpool_postgres::Client {
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Client::query_one(self, &stmt, parameters).await {
                Ok(result) => Ok(result),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            return Ok(
                                tokio_postgres::Client::query_one(self, &stmt, parameters).await?
                            );
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Client::query_opt(self, &stmt, parameters).await {
                Ok(result) => Ok(result),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            return Ok(
                                tokio_postgres::Client::query_opt(self, &stmt, parameters).await?
                            );
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Client::query(self, &stmt, parameters).await {
                Ok(result) => Ok(result),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            return Ok(
                                tokio_postgres::Client::query(self, &stmt, parameters).await?
                            );
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Client::execute(self, &stmt, parameters).await {
                Ok(_) => Ok(()),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            tokio_postgres::Client::execute(self, &stmt, parameters).await?;
                            return Ok(());
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }
}

impl Connection for deadpool_postgres::Transaction<'_> {
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Transaction::query_opt(self, &stmt, parameters).await {
                Ok(result) => result.ok_or_else(|| ErrorKind::RowNotFound.into()),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            return tokio_postgres::Transaction::query_opt(self, &stmt, parameters)
                                .await?
                                .ok_or_else(|| ErrorKind::RowNotFound.into());
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Transaction::query_opt(self, &stmt, parameters).await {
                Ok(result) => Ok(result),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            return Ok(tokio_postgres::Transaction::query_opt(
                                self, &stmt, parameters,
                            )
                            .await?);
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Transaction::query(self, &stmt, parameters).await {
                Ok(result) => Ok(result),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            return Ok(
                                tokio_postgres::Transaction::query(self, &stmt, parameters).await?
                            );
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        async move {
            let stmt = self.prepare_cached(query).await?;
            match tokio_postgres::Transaction::execute(self, &stmt, parameters).await {
                Ok(_) => Ok(()),
                Err(err) => {
                    if let Some(err) = err.as_db_error() {
                        if err.routine() == Some("RevalidateCachedQuery") {
                            tracing::warn!(%err, "clearing statement cache");
                            self.statement_cache.clear();
                            let stmt = self.prepare_cached(query).await?;
                            tokio_postgres::Transaction::execute(self, &stmt, parameters).await?;
                            return Ok(());
                        }
                    }
                    Err(err.into())
                }
            }
        }
    }
}

impl<C> Connection for &C
where
    C: Connection,
{
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a {
        (*self).query_one(query, parameters)
    }

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a {
        (*self).query_opt(query, parameters)
    }

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a {
        (*self).query(query, parameters)
    }

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        (*self).execute(query, parameters)
    }
}
