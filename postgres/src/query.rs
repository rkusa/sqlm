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
