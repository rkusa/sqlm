#![cfg_attr(nightly_column_names, feature(adt_const_params, unsized_const_params))]
#![cfg_attr(nightly_column_names, allow(incomplete_features))]
#![forbid(unsafe_code)]

//! An [`sql!`] macro to write compile-time checked database queries similar to how [`format!`]
//! works.
//!
//! # Example
//!
//! ```
//! use sqlm_postgres::{sql, Enum, FromRow, FromSql, ToSql};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let id: i64 = 1;
//! let user: User = sql!("SELECT * FROM users WHERE id = {id}").await?;
//!
//! #[derive(Debug, FromRow)]
//! struct User {
//!     id: i64,
//!     name: String,
//!     role: Role,
//! }
//!
//! #[derive(Debug, Default, FromSql, ToSql, Enum)]
//! #[postgres(name = "role")]
//! enum Role {
//!     #[default]
//!     #[postgres(name = "user")]
//!     User,
//!     #[postgres(name = "admin")]
//!     Admin,
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Usage
//! - Add `sqlm-postgres` to your dependencies
//! - Make the `DATABASE_URL` env variable available during compile time (e.g. via adding an `.env`
//!   file)
//! - Start using the [`sql!`] macro (no further setup necessary; a connection pool is automatically
//!   created for you)
//!
//! # Caveats
//! - Automatically creates a global connection pool for you with no way to opt out
//! - Compile-time checks cannot be disabled. Thus also requires database access on your CI.
//! - Does not know whether rows returned from Postgres are nullable and consequentially
//!   requires all types to implement [`Default::default`], which it falls back to if Postgres
//!   returns null.

// Necessary to have `::sqlm_postgres::` available in tests
#[cfg(test)]
extern crate self as sqlm_postgres;

mod connection;
pub mod error;
mod future;
#[doc(hidden)]
pub mod internal;
mod macros;
pub mod pool;
mod query;
mod row;
#[doc(hidden)]
pub mod types;

use std::marker::PhantomData;

pub use connection::{Connection, Session, Transaction};
use deadpool_postgres::ClientWrapper;
pub use error::Error;
pub use future::SqlFuture;
pub use macros::{Enum, FromRow, sql};
use query::Query;
pub use row::{FromRow, Row};
pub use tokio_postgres;
pub use tokio_postgres::types::{FromSql, ToSql};
pub use types::SqlType;

#[cfg(feature = "global_pool")]
static POOL: once_cell::sync::OnceCell<pool::Pool> = once_cell::sync::OnceCell::new();

/// Establish a database connection.
///
/// This function is automatically called when awaiting queries created with [`sql!`]. When first
/// called, a connection pool is created, which expects the env variable `DATABASE_URL` to be set.
///
/// When having multiple sequential queries, it is recommended to manually establish a connection
/// and pass it to [`sql!`] via [`Sql::run_with`].
///
/// # Examples
///
/// ```
/// # use sqlm_postgres::{sql, connect, Enum, FromRow, FromSql, ToSql};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let conn = connect().await?;
/// let name: String = sql!("SELECT name FROM users WHERE id = {id}", id = 1i64)
///     .run_with(conn)
///     .await?;
/// # Ok(())
/// # }
/// ```
/// ```
/// # use sqlm_postgres::{sql, connect, Enum, FromRow, FromSql, ToSql};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut conn = connect().await?;
/// let tx = conn.transaction().await?;
/// let name: String = sql!("SELECT name FROM users WHERE id = {id}", id = 1i64)
///     .run_with(&tx)
///     .await?;
/// tx.commit().await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "global_pool")]
#[tracing::instrument]
pub async fn connect() -> Result<Session, Error> {
    // Don't trace connect, as this would create an endless loop of connecting again and
    // again when persisting the connect trace!
    let pool = POOL.get_or_try_init(|| pool::Pool::from_env(4))?;
    pool.connect().await
}

/// The struct created by [`sql!`]; executed by calling `.await`.
pub struct Sql<'a, Cols, T> {
    // Fields need to be public so that they can be set by the macro invocation.
    #[doc(hidden)]
    pub query: &'static str,
    #[doc(hidden)]
    pub parameters: &'a [&'a (dyn ToSql + Sync)],
    #[doc(hidden)]
    pub transaction: Option<&'a Transaction<'a>>,
    #[doc(hidden)]
    pub connection: Option<&'a ClientWrapper>,
    #[doc(hidden)]
    pub marker: PhantomData<(Cols, T)>,
}

impl<'a, Cols, T> Sql<'a, Cols, T> {
    /// Manually pass a connection or transaction to a query created with [`sql!`].
    ///
    /// See [`connect`] for examples.
    pub fn run_with(self, conn: impl Connection + 'a) -> SqlFuture<'a, T>
    where
        T: Query<Cols> + Send + Sync + 'a,
        Cols: Send + Sync + 'a,
    {
        SqlFuture::with_connection(self, conn)
    }
}
