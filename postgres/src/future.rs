use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::row::FromRow;
use crate::{connect, Error, Sql};

// impl<'a, Cols, T, const NAMES: usize> IntoFuture for Sql<'a, Cols, Vec<T>, NAMES>
// where
//     T: FromRow<Cols, NAMES>,
// {
//     type Output = Result<T, Error>;
//     type IntoFuture = SqlFuture<'a, Vec<T>>;

//     fn into_future(self) -> Self::IntoFuture {
//         SqlFuture {
//             future: Box::pin(async move {
//                 let conn = connect().await?;
//                 let stmt = conn.prepare_cached(self.query).await?;
//                 // for (i, p) in self.parameters.iter().enumerate() {
//                 //     // stmt.raw_bind_parameter(i + 1, p)?;
//                 //     stmt.p
//                 // }
//                 let rows = conn.query(&stmt, self.parameters).await?;
//                 // let rows = client.query(&stmt, &[&i]).await.unwrap();
//                 todo!()
//             }),
//             marker: PhantomData,
//         }
//     }
// }

impl<'a, Cols, T> IntoFuture for Sql<'a, Cols, T>
where
    T: FromRow<Cols>,
{
    type Output = Result<T, Error>;
    type IntoFuture = SqlFuture<'a, T>;

    fn into_future(self) -> Self::IntoFuture {
        SqlFuture {
            future: Box::pin(async move {
                let conn = connect().await?;
                let stmt = conn.prepare_cached(self.query).await?;
                let row = conn.query_one(&stmt, self.parameters).await?;
                Ok(T::from_row(row.into())?)
            }),
            marker: PhantomData,
        }
    }
}

pub struct SqlFuture<'a, T> {
    future: Pin<Box<dyn Future<Output = Result<T, Error>> + 'a>>,
    marker: PhantomData<&'a ()>,
}

impl<'a, T> Future for SqlFuture<'a, T> {
    type Output = Result<T, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
