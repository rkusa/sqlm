use std::future::Future;
use std::pin::Pin;

use tokio_postgres::types::{FromSqlOwned, ToSql};

use crate::types::{Array, Bytea, Primitive, SqlType, Struct};
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

impl Query<Primitive<Bytea>> for Vec<u8> {
    fn query<'a>(
        sql: &'a Sql<'a, Primitive<Bytea>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = conn.query_one(sql.query, sql.parameters).await?;
            Ok(row.try_get(0)?)
        })
    }
}

impl<T> Query<Primitive<T::Type>> for Option<T>
where
    T: SqlType + FromSqlOwned + ToSql + Send + Sync + 'static,
    T::Type: Send + Sync + 'static,
{
    fn query<'a>(
        sql: &'a Sql<'a, Primitive<T::Type>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = conn.query_opt(sql.query, sql.parameters).await?;
            match row {
                Some(row) => Ok(row.try_get::<'_, _, Option<T>>(0)?),
                None => Ok(None),
            }
        })
    }
}

impl Query<Primitive<Bytea>> for Option<Vec<u8>> {
    fn query<'a>(
        sql: &'a Sql<'a, Primitive<Bytea>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = conn.query_opt(sql.query, sql.parameters).await?;
            match row {
                Some(row) => Ok(row.try_get::<'_, _, Self>(0)?),
                None => Ok(None),
            }
        })
    }
}

impl<T> Query<Primitive<T::Type>> for Vec<T>
where
    T: SqlType + FromSqlOwned + ToSql + Send + Sync + 'static,
    T::Type: Send + Sync + 'static,
{
    fn query<'a>(
        sql: &'a Sql<'a, Primitive<T::Type>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let rows = conn.query(sql.query, sql.parameters).await?;
            rows.into_iter()
                .map(|row| row.try_get(0).map_err(Error::from))
                .collect()
        })
    }
}

impl Query<Primitive<Bytea>> for Vec<Vec<u8>> {
    fn query<'a>(
        sql: &'a Sql<'a, Primitive<Bytea>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let rows = conn.query(sql.query, sql.parameters).await?;
            rows.into_iter()
                .map(|row| row.try_get(0).map_err(Error::from))
                .collect()
        })
    }
}

impl<T> Query<Array<Vec<T::Type>>> for Vec<T>
where
    T: SqlType + FromSqlOwned + ToSql + Send + Sync + 'static,
    T::Type: Send + Sync + 'static,
{
    fn query<'a>(
        sql: &'a Sql<'a, Array<Vec<T::Type>>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = conn.query_one(sql.query, sql.parameters).await?;
            Ok(row.try_get(0)?)
        })
    }
}

impl Query<Array<Vec<Bytea>>> for Vec<Vec<u8>> {
    fn query<'a>(
        sql: &'a Sql<'a, Array<Vec<Bytea>>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let row = conn.query_one(sql.query, sql.parameters).await?;
            Ok(row.try_get(0)?)
        })
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
