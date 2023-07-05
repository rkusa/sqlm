use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::query::Query;
use crate::{Error, Sql};

impl<'a, Cols, T> IntoFuture for Sql<'a, Cols, T>
where
    T: Query<Cols> + 'a,
    Cols: 'a,
{
    type Output = Result<T, Error>;
    type IntoFuture = SqlFuture<'a, T>;

    fn into_future(self) -> Self::IntoFuture {
        SqlFuture {
            future: Box::pin(async move {
                let mut i = 1;
                loop {
                    match T::query(&self).await {
                        Ok(r) => return Ok(r),
                        Err(Error::Postgres(err)) if err.is_closed() && i <= 1 => {
                            // retry once if connection is closed (might have received a closed one
                            // from the connection pool)
                            i += 1;
                            continue;
                        }
                        Err(err) => return Err(err),
                    }
                }
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
