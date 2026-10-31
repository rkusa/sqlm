// TODO: remove once Rust's async lifetime in trait story got improved
#![allow(clippy::manual_async_fn)]

use std::future::Future;

use tokio_postgres::Row;
use tokio_postgres::types::ToSql;

use crate::error::Error;

/// A database transaction.
pub struct Transaction<'t>(pub(crate) deadpool_postgres::Transaction<'t>);

/// An asynchronous PostgreSQL client (basically a non-transactional connection).
pub struct Session(pub(crate) deadpool_postgres::Client);

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

impl Session {
    pub fn into_inner(self) -> deadpool_postgres::Client {
        self.0
    }

    pub async fn transaction(&mut self) -> Result<Transaction<'_>, Error> {
        self.0
            .transaction()
            .await
            .map(Transaction)
            .map_err(Error::from)
    }

    pub fn metrics(&self) -> &deadpool_postgres::Metrics {
        deadpool_postgres::Object::metrics(&self.0)
    }
}

impl<'t> Transaction<'t> {
    pub fn into_inner(self) -> deadpool_postgres::Transaction<'t> {
        self.0
    }

    pub async fn commit(self) -> Result<(), Error> {
        self.0.commit().await.map_err(Error::from)
    }

    pub async fn rollback(self) -> Result<(), Error> {
        self.0.rollback().await.map_err(Error::from)
    }
}

/// Query execution via deadpool's statement cache, shared by the pooled client and transaction.
mod cached {
    use std::future::Future;

    use deadpool_postgres::{GenericClient, StatementCache};
    use tokio_postgres::types::ToSql;
    use tokio_postgres::{Row, Statement};

    use crate::error::{Error, ErrorKind};

    /// A pooled client or transaction that caches prepared statements.
    pub(super) trait CachedClient: GenericClient {
        fn statement_cache(&self) -> &StatementCache;
    }

    impl CachedClient for deadpool_postgres::Client {
        fn statement_cache(&self) -> &StatementCache {
            &self.statement_cache
        }
    }

    impl CachedClient for deadpool_postgres::Transaction<'_> {
        fn statement_cache(&self) -> &StatementCache {
            &self.statement_cache
        }
    }

    /// Prepares `query` via the statement cache and runs `f` with the resulting statement. If
    /// Postgres rejects the cached statement as stale (e.g. after a schema change), the cache is
    /// cleared and the query is prepared and run once more.
    async fn with_statement<C, F, Fut, R>(client: &C, query: &str, f: F) -> Result<R, Error>
    where
        C: CachedClient,
        F: Fn(Statement) -> Fut,
        Fut: Future<Output = Result<R, tokio_postgres::Error>>,
    {
        let stmt = client.prepare_cached(query).await?;
        match f(stmt).await {
            Err(err)
                if err
                    .as_db_error()
                    .is_some_and(|err| err.routine() == Some("RevalidateCachedQuery")) =>
            {
                tracing::warn!(%err, "clearing statement cache");
                client.statement_cache().clear();
                let stmt = client.prepare_cached(query).await?;
                Ok(f(stmt).await?)
            }
            result => Ok(result?),
        }
    }

    pub(super) async fn query_one<C: CachedClient>(
        client: &C,
        query: &str,
        parameters: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error> {
        query_opt(client, query, parameters)
            .await?
            .ok_or_else(|| ErrorKind::RowNotFound.into())
    }

    pub(super) async fn query_opt<C: CachedClient>(
        client: &C,
        query: &str,
        parameters: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error> {
        with_statement(client, query, |stmt| async move {
            client.query_opt(&stmt, parameters).await
        })
        .await
    }

    pub(super) async fn query<C: CachedClient>(
        client: &C,
        query: &str,
        parameters: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error> {
        with_statement(client, query, |stmt| async move {
            client.query(&stmt, parameters).await
        })
        .await
    }

    pub(super) async fn execute<C: CachedClient>(
        client: &C,
        query: &str,
        parameters: &[&(dyn ToSql + Sync)],
    ) -> Result<(), Error> {
        with_statement(client, query, |stmt| async move {
            client.execute(&stmt, parameters).await
        })
        .await?;
        Ok(())
    }
}

impl Connection for deadpool_postgres::Client {
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a {
        cached::query_one(self, query, parameters)
    }

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a {
        cached::query_opt(self, query, parameters)
    }

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a {
        cached::query(self, query, parameters)
    }

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        cached::execute(self, query, parameters)
    }
}

impl Connection for deadpool_postgres::Transaction<'_> {
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a {
        cached::query_one(self, query, parameters)
    }

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a {
        cached::query_opt(self, query, parameters)
    }

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a {
        cached::query(self, query, parameters)
    }

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        cached::execute(self, query, parameters)
    }
}

impl Connection for Session {
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a {
        Connection::query_one(&self.0, query, parameters)
    }

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a {
        Connection::query_opt(&self.0, query, parameters)
    }

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a {
        Connection::query(&self.0, query, parameters)
    }

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        Connection::execute(&self.0, query, parameters)
    }
}

impl Connection for Transaction<'_> {
    fn query_one<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Row, Error>> + Send + 'a {
        Connection::query_one(&self.0, query, parameters)
    }

    fn query_opt<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Option<Row>, Error>> + Send + 'a {
        Connection::query_opt(&self.0, query, parameters)
    }

    fn query<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<Vec<Row>, Error>> + Send + 'a {
        Connection::query(&self.0, query, parameters)
    }

    fn execute<'a>(
        &'a self,
        query: &'a str,
        parameters: &'a [&'a (dyn ToSql + Sync)],
    ) -> impl Future<Output = Result<(), Error>> + Send + 'a {
        Connection::execute(&self.0, query, parameters)
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
