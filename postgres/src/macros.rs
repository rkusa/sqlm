/// Creates a parameterized, compile-time checked database query that accepts parameters similar to
/// the [`format!`] macro.
///
/// The compile-time checks requrire a database connection, expecting a `DATABASE_URL` env to be set
/// accordingly.
///
/// The returned type can either be a struct (that implements [`FromRow`]), a literal (string,
/// interger, ...), or a [`Vec`] or [`Option`] of the former.
///
/// A connection is automatically established, but also be explicitly set via
/// [`Sql::run_with`].
///
/// # Examples
///
/// ```
/// # use sqlm_postgres::sql;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let name: String = sql!("SELECT name FROM users WHERE id = {id}", id = 1i64).await?;
/// # Ok(())
/// # }
/// ```
///
/// [`FromRow`]: super::FromRow
/// [`Sql::run_with`]: super::Sql::run_with
pub use sqlm_postgres_macros::sql;
/// A derive necessary to support compile checks between Postgres and Rust enums.
///
/// In addition, enums also need to implement `tokio_postgres`'s [`FromSql`] and [`ToSql`], so it
/// can be read from and written to Postgres.
///
/// # Example
/// ```
/// use sqlm_postgres::{Enum, FromSql, ToSql};
/// #[derive(Debug, Default, FromSql, ToSql, Enum)]
/// #[postgres(name = "role")]
/// enum Role {
///     #[default]
///     #[postgres(name = "user")]
///     User,
///     #[postgres(name = "admin")]
///     Admin,
/// }
/// ```
///
/// [`FromSql`]: crate::FromSql
/// [`ToSql`]: crate::ToSql
pub use sqlm_postgres_macros::Enum;
/// Derive [`FromRow`] for a struct, required read a query result into a struct.
///
/// Each struct property must have a [`Default::default`] implementation (used for null values; you
/// can of course use [`Option`] as its default is simply [`None`]).
/// Alternatively, the default value can be set using a `#[sqlm(default = ...)]` attribute.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "time")]
/// #[derive(sqlm_postgres::FromRow)]
/// struct User {
///     id: i64,
///     name: String,
///     #[sqlm(default = time::OffsetDateTime::UNIX_EPOCH)]
///     created_at: time::OffsetDateTime,
/// }
/// ```
///
/// [`FromRow`]: trait@crate::FromRow
pub use sqlm_postgres_macros::FromRow;
