use std::future::Future;
use std::pin::Pin;

use crate::{connect, Error, FromRow, Sql};

pub trait Query<Cols>: Sized {
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + 'a>>;
}

impl<T, Cols> Query<Cols> for Vec<T>
where
    T: FromRow<Cols>,
{
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + 'a>> {
        Box::pin(async move {
            let conn = connect().await?;
            let stmt = conn.prepare_cached(sql.query).await?;
            let rows = conn.query(&stmt, sql.parameters).await?;
            rows.into_iter()
                .map(|row| FromRow::<Cols>::from_row(row.into()).map_err(Error::from))
                .collect()
        })
    }
}

impl<T, Cols> Query<Cols> for Option<T>
where
    T: FromRow<Cols>,
{
    fn query<'a>(
        sql: &'a Sql<'a, Cols, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + 'a>> {
        Box::pin(async move {
            let conn = connect().await?;
            let stmt = conn.prepare_cached(sql.query).await?;
            let row = conn.query_opt(&stmt, sql.parameters).await?;
            match row {
                Some(row) => Ok(Some(FromRow::<Cols>::from_row(row.into())?)),
                None => Ok(None),
            }
        })
    }
}

#[cfg(feature = "comptime")]
macro_rules! impl_query_scalar {
    ($ty:tt) => {
        impl<Cols> Query<Cols> for $ty
        where
            Cols: crate::row::HasScalar<Self>,
        {
            fn query<'a>(
                sql: &'a Sql<'a, Cols, Self>,
            ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + 'a>> {
                Box::pin(async move {
                    let conn = connect().await?;
                    let stmt = conn.prepare_cached(sql.query).await?;
                    let row = conn.query_one(&stmt, sql.parameters).await?;
                    Ok(row.try_get(0)?)
                })
            }
        }
    };
}

#[cfg(not(feature = "comptime"))]
macro_rules! impl_query_scalar {
    ($ty:tt) => {
        impl<Cols> Query<Cols> for $ty {
            fn query<'a>(
                sql: &'a Sql<'a, Cols, Self>,
            ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + 'a>> {
                Box::pin(async move {
                    let conn = connect().await?;
                    let stmt = conn.prepare_cached(sql.query).await?;
                    let row = conn.query_one(&stmt, sql.parameters).await?;
                    Ok(row.try_get(0)?)
                })
            }
        }
    };
}

impl_query_scalar!(i64);
impl_query_scalar!(bool);
impl_query_scalar!(String);
