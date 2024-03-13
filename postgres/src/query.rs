use std::future::Future;
use std::pin::Pin;

use tokio_postgres::types::{FromSqlOwned, ToSql};

use crate::types::{Primitive, SqlType, Struct};
use crate::{Error, FromRow, Sql};

pub trait Query<Cols>: Sized {
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>;
}

impl<T> Query<Primitive<T::Type>> for T
where
    T: SqlType + FromSqlOwned + ToSql + Send + Sync + 'static,
    T::Type: Send + Sync + 'static,
{
    fn query<'a>(
        sql: &'a Sql<'a, Primitive<T::Type>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        T::query_literal(sql, conn)
    }
}

impl<T, Cols> Query<Struct<Cols>> for T
where
    Cols: Send + Sync,
    T: FromRow<Struct<Cols>> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, Struct<Cols>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = conn.query_one(sql.query, sql.parameters).await?;
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
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = conn.query_opt(sql.query, sql.parameters).await?;
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
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let rows = conn.query(sql.query, sql.parameters).await?;
            rows.into_iter()
                .map(|row| FromRow::<Struct<Cols>>::from_row(row.into()).map_err(Error::from))
                .collect()
        })
    }
}

impl Query<()> for () {
    fn query<'a>(
        sql: &'a Sql<'a, (), Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            conn.execute(sql.query, sql.parameters).await?;
            Ok(())
        })
    }
}
