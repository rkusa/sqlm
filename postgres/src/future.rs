use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::query::Query;
use crate::{Error, Sql};

impl<'a, Cols, T> IntoFuture for Sql<'a, Cols, T>
where
    T: Query<Cols>,
{
    type Output = Result<T, Error>;
    type IntoFuture = SqlFuture<'a, T>;

    fn into_future(self) -> Self::IntoFuture {
        SqlFuture {
            future: T::query(self),
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
