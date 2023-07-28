use std::future::Future;
use std::pin::Pin;

use crate::{connect, Error, FromRow, Literal, Sql, SqlType, Struct};

pub trait Query<Cols>: Sized {
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>;
}

#[cfg(feature = "comptime")]
mod comptime {
    use super::*;

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

    impl<T, Cols> Query<Literal<Cols>> for Option<T>
    where
        Cols: Send + Sync,
        T: tokio_postgres::types::FromSqlOwned + Send + Sync,
    {
        fn query<'a>(
            sql: &'a Sql<'a, Literal<Cols>, Self>,
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

    impl<T, Cols> Query<Literal<Cols>> for Vec<T>
    where
        Cols: Send + Sync,
        T: tokio_postgres::types::FromSqlOwned + Send + Sync,
    {
        fn query<'a>(
            sql: &'a Sql<'a, Literal<Cols>, Self>,
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
}

#[cfg(not(feature = "comptime"))]
mod not_comptime {
    use super::*;

    impl<T> Query<Struct<Self>> for Option<T>
    where
        T: FromRow<T> + Send + Sync,
    {
        fn query<'a>(
            sql: &'a Sql<'a, Struct<Self>, Self>,
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
                    Some(row) => Ok(Some(FromRow::<T>::from_row(row.into())?)),
                    None => Ok(None),
                }
            })
        }
    }

    impl<T> Query<Literal<Option<T>>> for Option<T>
    where
        T: tokio_postgres::types::FromSqlOwned + Send + Sync,
    {
        fn query<'a>(
            sql: &'a Sql<'a, Literal<Option<T>>, Self>,
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

    impl<T> Query<Struct<Self>> for Vec<T>
    where
        T: FromRow<T> + Send + Sync,
    {
        fn query<'a>(
            sql: &'a Sql<'a, Struct<Self>, Self>,
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
                    .map(|row| FromRow::<T>::from_row(row.into()).map_err(Error::from))
                    .collect()
            })
        }
    }

    impl<T> Query<Literal<Vec<T>>> for Vec<T>
    where
        T: tokio_postgres::types::FromSqlOwned + Send + Sync,
    {
        fn query<'a>(
            sql: &'a Sql<'a, Literal<Vec<T>>, Self>,
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
        impl SqlType for $ty {
            type Type = Self;
        }

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
        impl SqlType for $ty {
            type Type = Self;
        }

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
