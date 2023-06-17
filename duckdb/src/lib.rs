use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{env, error, fmt, mem};

pub use duckdb;
use duckdb::{CachedStatement, Connection, DuckdbConnectionManager};
pub use duckdb::{Row, Statement, ToSql};
use once_cell::sync::OnceCell;
use r2d2::{Pool, PooledConnection};
pub use sqlm_macros::sql;
use tokio::task::{self, JoinHandle};

static POOL: OnceCell<Pool<DuckdbConnectionManager>> = OnceCell::new();

pub fn connect() -> Result<PooledConnection<DuckdbConnectionManager>, Error> {
    let pool = POOL.get_or_try_init(|| {
        Ok::<_, Error>(
            Pool::new(
                DuckdbConnectionManager::file(
                    env::var("DATABASE_PATH")
                        .as_deref()
                        .unwrap_or("./db.duckdb"),
                )
                .unwrap(),
            )
            .unwrap(),
        )
    })?;
    Ok(pool.get()?)
}

pub struct Sql<'a, T = ()> {
    // TODO: not pub?
    pub query: &'static str,
    pub parameters: &'a [&'a (dyn ToSql + Send + Sync)],
    pub marker: PhantomData<T>,
}

impl<'a, T> Sql<'a, T>
where
    T: FromStatement,
{
    pub fn block_in_place(self) -> Result<T, Error> {
        let conn = connect()?;
        self.block_in_place_with(&conn)
    }

    pub fn block_in_place_with(self, conn: &Connection) -> Result<T, Error> {
        let mut stmt = conn.prepare_cached(self.query)?;
        for (i, p) in self.parameters.iter().enumerate() {
            stmt.raw_bind_parameter(i + 1, p)?;
        }
        stmt.raw_execute()?;
        let result = T::from_statement(stmt)?;
        Ok(result)
    }
}

pub trait FromStatement {
    fn from_statement(stmt: CachedStatement<'_>) -> Result<Self, Error>
    where
        Self: Sized;
}

pub trait FromRow {
    fn from_row(row: &Row<'_>) -> Result<Self, duckdb::Error>
    where
        Self: Sized;
}

impl<'a, T> IntoFuture for Sql<'a, T>
where
    T: FromStatement + Send + 'static,
{
    type Output = Result<T, Error>;
    type IntoFuture = SqlFuture<'a, T>;

    fn into_future(self) -> Self::IntoFuture {
        // TODO: safe?
        let static_self = unsafe { mem::transmute::<Sql<'a, T>, Sql<'static, T>>(self) };
        SqlFuture {
            join: Box::pin(task::spawn_blocking(move || static_self.block_in_place())),
            marker: PhantomData,
        }
    }
}

pub struct SqlFuture<'a, T> {
    join: Pin<Box<JoinHandle<Result<T, Error>>>>,
    marker: PhantomData<&'a ()>,
}

impl<'a, T> Future for SqlFuture<'a, T>
where
    T: FromStatement,
{
    type Output = Result<T, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.join.as_mut().poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(err)) => Poll::Ready(Err(err.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> FromStatement for Vec<T>
where
    T: FromRow,
{
    fn from_statement(mut stmt: CachedStatement<'_>) -> Result<Self, Error> {
        let rows = stmt.raw_query();
        rows.mapped(|row| T::from_row(row))
            .collect::<Result<Vec<T>, duckdb::Error>>()
            .map_err(Error::DuckDb)
    }
}

impl FromStatement for () {
    fn from_statement(_stmt: CachedStatement<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(())
    }
}

impl<T> FromStatement for T
where
    T: FromRow,
{
    fn from_statement(mut stmt: CachedStatement<'_>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let mut rows = stmt.raw_query();
        match rows.next()? {
            Some(row) => Ok(T::from_row(row)?),
            None => Err(duckdb::Error::QueryReturnedNoRows.into()),
        }
    }
}

impl FromRow for String {
    fn from_row(row: &Row<'_>) -> Result<Self, duckdb::Error> {
        row.get(0)
    }
}

#[derive(Debug)]
pub enum Error {
    DuckDb(duckdb::Error),
    Pool(r2d2::Error),
    Join(task::JoinError),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::DuckDb(err) => Some(err),
            Error::Pool(err) => Some(err),
            Error::Join(err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // DbError::DuckDb(_) => f.write_str("error in db connection"),
            Error::DuckDb(err) => err.fmt(f),
            Error::Pool(err) => err.fmt(f),
            Error::Join(_) => f.write_str("error in db task"),
        }
    }
}

impl From<duckdb::Error> for Error {
    fn from(err: duckdb::Error) -> Self {
        Self::DuckDb(err)
    }
}

impl From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Self {
        Self::Pool(err)
    }
}

impl From<task::JoinError> for Error {
    fn from(err: task::JoinError) -> Self {
        Self::Join(err)
    }
}
