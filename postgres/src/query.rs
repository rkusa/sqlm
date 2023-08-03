use std::future::Future;
use std::pin::Pin;

use tokio_postgres::types::{FromSqlOwned, ToSql};

use crate::types::{Literal, SqlType, Struct};
use crate::{Error, FromRow, Sql};

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
            let row = sql.query_one().await?;
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
            let row = sql.query_opt().await?;
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
            let rows = sql.query().await?;
            rows.into_iter()
                .map(|row| FromRow::<Struct<Cols>>::from_row(row.into()).map_err(Error::from))
                .collect()
        })
    }
}

impl Query<()> for () {
    fn query<'a>(
        sql: &'a Sql<'a, (), Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            sql.execute().await?;
            Ok(())
        })
    }
}
