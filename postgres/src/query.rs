use std::future::Future;
use std::pin::Pin;

use crate::{connect, Error, FromRow, Row, Sql};

pub trait Query<Cols>: Sized {
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>;
}

impl<T, Cols> Query<Cols> for Vec<T>
where
    Cols: Send + Sync,
    T: FromRow<Cols> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
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
                .map(|row| FromRow::<Cols>::from_row(row.into()).map_err(Error::from))
                .collect()
        })
    }
}

impl<T, Cols> Query<Cols> for Option<T>
where
    Cols: Send + Sync,
    T: FromRow<Cols> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
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
                Some(row) => Ok(Some(FromRow::<Cols>::from_row(row.into())?)),
                None => Ok(None),
            }
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

#[cfg(feature = "comptime")]
macro_rules! impl_query_scalar {
    ($ty:path) => {
        impl<Cols> Query<Cols> for $ty
        where
            Cols: crate::row::HasScalar<Self> + Send + Sync,
        {
            fn query<'a>(
                sql: &'a Sql<'a, Cols, Self>,
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
                    Ok(row.try_get(0)?)
                })
            }
        }

        impl<Cols> FromRow<Cols> for $ty
        where
            Cols: crate::row::HasScalar<Self> + Send + Sync,
        {
            fn from_row(row: Row<Cols>) -> Result<Self, tokio_postgres::Error> {
                row.try_get(0)
            }
        }
    };
}

#[cfg(not(feature = "comptime"))]
macro_rules! impl_query_scalar {
    ($ty:path) => {
        impl<Cols> Query<Cols> for $ty
        where
            Cols: Send + Sync,
        {
            fn query<'a>(
                sql: &'a Sql<'a, Cols, Self>,
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
                    Ok(row.try_get(0)?)
                })
            }
        }

        impl<Cols> FromRow<Cols> for $ty
        where
            Cols: Send + Sync,
        {
            fn from_row(row: Row<Cols>) -> Result<Self, tokio_postgres::Error> {
                row.try_get(0)
            }
        }
    };
}

impl_query_scalar!(i64);
impl_query_scalar!(bool);
impl_query_scalar!(String);
#[cfg(feature = "json")]
impl_query_scalar!(serde_json::Value);
