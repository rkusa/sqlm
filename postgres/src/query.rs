use std::future::Future;
use std::pin::Pin;

use tokio_postgres::types::{FromSqlOwned, ToSql};

use crate::types::{Literal, SqlType, Struct};
use crate::{connect, Error, FromRow, Sql};

pub trait Query<Cols>: Sized {
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>;
}

impl<T> Query<Literal<T::Type>> for T
where
    T: SqlType + FromSqlOwned + ToSql + Send + Sync + 'static,
    T::Type: Send + Sync + 'static,
{
    fn query<'a>(
        sql: &'a Sql<'a, Literal<T::Type>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        T::query_literal(sql)
    }
}

impl<T, Cols> Query<Struct<Cols>> for T
where
    Cols: Send + Sync,
    T: FromRow<Struct<Cols>> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, Struct<Cols>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = if let Some(tx) = sql.transaction {
                let stmt = tx.prepare_cached(sql.query).await?;
                tx.query_one(&stmt, sql.parameters).await?
            } else {
                let conn = connect().await?;
                let stmt = conn.prepare_cached(sql.query).await?;
                conn.query_one(&stmt, sql.parameters).await?
            };
            Ok(FromRow::<Struct<Cols>>::from_row(row.into())?)
        })
    }
}

impl<T, Cols> Query<Struct<Cols>> for Option<T>
where
    Cols: Send + Sync,
    T: FromRow<Struct<Cols>> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, Struct<Cols>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = if let Some(tx) = sql.transaction {
                let stmt = tx.prepare_cached(sql.query).await?;
                tx.query_opt(&stmt, sql.parameters).await?
            } else {
                let conn = connect().await?;
                let stmt = conn.prepare_cached(sql.query).await?;
                conn.query_opt(&stmt, sql.parameters).await?
            };
            match row {
                Some(row) => Ok(Some(FromRow::<Struct<Cols>>::from_row(row.into())?)),
                None => Ok(None),
            }
        })
    }
}

impl<T, Cols> Query<Struct<Cols>> for Vec<T>
where
    Cols: Send + Sync,
    T: FromRow<Struct<Cols>> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, Struct<Cols>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let rows = if let Some(tx) = sql.transaction {
                let stmt = tx.prepare_cached(sql.query).await?;
                tx.query(&stmt, sql.parameters).await?
            } else {
                let conn = connect().await?;
                let stmt = conn.prepare_cached(sql.query).await?;
                conn.query(&stmt, sql.parameters).await?
            };
            rows.into_iter()
                .map(|row| FromRow::<Struct<Cols>>::from_row(row.into()).map_err(Error::from))
                .collect()
        })
    }
}

impl<Cols> Query<Cols> for ()
where
    Cols: Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(tx) = sql.transaction {
                let stmt = tx.prepare_cached(sql.query).await?;
                tx.execute(&stmt, sql.parameters).await?;
            } else {
                let conn = connect().await?;
                let stmt = conn.prepare_cached(sql.query).await?;
                conn.execute(&stmt, sql.parameters).await?;
            }
            Ok(())
        })
    }
}
