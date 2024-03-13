use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use tracing::Instrument;

use crate::error::ErrorKind;
use crate::query::Query;
use crate::{Error, Sql};

impl<'a, Cols, T> IntoFuture for Sql<'a, Cols, T>
where
    T: Query<Cols> + Send + Sync + 'a,
    Cols: Send + Sync + 'a,
{
    type Output = Result<T, Error>;
    type IntoFuture = SqlFuture<'a, T>;

    fn into_future(self) -> Self::IntoFuture {
        SqlFuture::new(self)
    }
}

pub struct SqlFuture<'a, T> {
    future: Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>,
    marker: PhantomData<&'a ()>,
}

impl<'a, T> SqlFuture<'a, T> {
    pub fn new<Cols>(sql: Sql<'a, Cols, T>) -> Self
    where
        T: Query<Cols> + Send + Sync + 'a,
        Cols: Send + Sync + 'a,
    {
        let span =
            tracing::debug_span!("sql query", query = sql.query, parameters = ?sql.parameters);
        let start = Instant::now();

        SqlFuture {
            future: Box::pin(
                // Note: changes here must be applied to `with_connection` below too!
                async move {
                    let mut i = 1;
                    loop {
                        let conn = super::connect().await?;
                        match T::query(&sql, &conn).await {
                            Ok(r) => {
                                let elapsed = start.elapsed();
                                tracing::trace!(?elapsed, "sql query finished");
                                return Ok(r);
                            }
                            Err(Error {
                                kind: ErrorKind::Postgres(err),
                                ..
                            }) if err.is_closed() && i <= 5 => {
                                // retry pool size + 1 times if connection is closed (might have
                                // received a closed one from the connection pool)
                                i += 1;
                                tracing::trace!("retry due to connection closed error");
                                continue;
                            }
                            Err(err) => {
                                return Err(err);
                            }
                        }
                    }
                }
                .instrument(span),
            ),
            marker: PhantomData,
        }
    }

    pub fn with_connection<Cols>(sql: Sql<'a, Cols, T>, conn: impl super::Connection + 'a) -> Self
    where
        T: Query<Cols> + Send + Sync + 'a,
        Cols: Send + Sync + 'a,
    {
        let span =
            tracing::debug_span!("sql query", query = sql.query, parameters = ?sql.parameters);
        let start = Instant::now();

        SqlFuture {
            future: Box::pin(
                // Note: changes here must be applied to `bew` above too!
                async move {
                    let mut i = 1;
                    loop {
                        match T::query(&sql, &conn).await {
                            Ok(r) => {
                                let elapsed = start.elapsed();
                                tracing::trace!(?elapsed, "sql query finished");
                                return Ok(r);
                            }
                            Err(Error {
                                kind: ErrorKind::Postgres(err),
                                ..
                            }) if err.is_closed() && i <= 5 => {
                                // retry pool size + 1 times if connection is closed (might have
                                // received a closed one from the connection pool)
                                i += 1;
                                tracing::trace!("retry due to connection closed error");
                                continue;
                            }
                            Err(err) => {
                                return Err(err);
                            }
                        }
                    }
                }
                .instrument(span),
            ),
            marker: PhantomData,
        }
    }
}

impl<'a, T> Future for SqlFuture<'a, T> {
    type Output = Result<T, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
