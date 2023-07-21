use std::future::Future;
use std::pin::Pin;

use crate::{connect, Error, FromRow, Sql};

pub trait Query<Cols>: Sized {
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>;
}

#[cfg(feature = "comptime")]
impl<T, Cols> Query<crate::Struct<Cols>> for Option<T>
where
    Cols: Send + Sync,
    T: FromRow<crate::Struct<Cols>> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, crate::Struct<Cols>, Self>,
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
                Some(row) => Ok(Some(FromRow::<crate::Struct<Cols>>::from_row(row.into())?)),
                None => Ok(None),
            }
        })
    }
}

#[cfg(feature = "comptime")]
impl<T, Cols> Query<crate::Literal<Cols>> for Option<T>
where
    Cols: Send + Sync,
    T: tokio_postgres::types::FromSqlOwned + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, crate::Literal<Cols>, Self>,
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
                Some(row) => Ok(row.try_get::<'_, _, Option<T>>(0)?),
                None => Ok(None),
            }
        })
    }
}

#[cfg(feature = "comptime")]
impl<T, Cols> Query<crate::Struct<Cols>> for Vec<T>
where
    Cols: Send + Sync,
    T: FromRow<crate::Struct<Cols>> + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, crate::Struct<Cols>, Self>,
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
                .map(|row| {
                    FromRow::<crate::Struct<Cols>>::from_row(row.into()).map_err(Error::from)
                })
                .collect()
        })
    }
}

#[cfg(feature = "comptime")]
impl<T, Cols> Query<crate::Literal<Cols>> for Vec<T>
where
    Cols: Send + Sync,
    T: tokio_postgres::types::FromSqlOwned + Send + Sync,
{
    fn query<'a>(
        sql: &'a Sql<'a, crate::Literal<Cols>, Self>,
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
        impl Query<crate::Literal<$ty>> for $ty {
            fn query<'a>(
                sql: &'a Sql<'a, crate::Literal<$ty>, Self>,
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
    };
}

#[cfg(not(feature = "comptime"))]
macro_rules! impl_query_scalar {
    ($ty:path) => {
        impl Query<()> for $ty {
            fn query<'a>(
                sql: &'a Sql<'a, (), Self>,
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

        impl Query<()> for Option<$ty> {
            fn query<'a>(
                sql: &'a Sql<'a, (), Self>,
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
                        Some(row) => Ok(row.try_get::<'_, _, Option<$ty>>(0)?),
                        None => Ok(None),
                    }
                })
            }
        }

        impl Query<()> for Vec<$ty> {
            fn query<'a>(
                sql: &'a Sql<'a, (), Self>,
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

        impl FromRow<()> for $ty {
            fn from_row(row: crate::row::Row<()>) -> Result<Self, tokio_postgres::Error> {
                row.try_get(0)
            }
        }
    };
}

impl_query_scalar!(i32);
impl_query_scalar!(i64);
impl_query_scalar!(bool);
impl_query_scalar!(String);
#[cfg(feature = "json")]
impl_query_scalar!(serde_json::Value);
#[cfg(feature = "time")]
impl_query_scalar!(time::OffsetDateTime);
#[cfg(feature = "uuid")]
impl_query_scalar!(uuid::Uuid);
