use std::marker::PhantomData;
use std::ops::Deref;

/// A row of data returned from Postgres.
pub struct Row<Cols> {
    row: tokio_postgres::Row,
    marker: PhantomData<Cols>,
}

/// A trait for types that can be created from a [`Row`].
///
/// This is usually derived via [`FromRow`] and not implemented manually.
///
/// [`FromRow`]: `derive@crate::FromRow`
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be read from a row",
    label = "does not implement `FromRow`",
    note = "derive `FromRow` for `{Self}`"
)]
pub trait FromRow: Sized {
    /// The struct's fields as a list of columns, compared against the columns returned by a query
    /// (see [`Compatible`]).
    ///
    /// [`Compatible`]: crate::types::Compatible
    type Cols;

    fn from_row(row: Row<Self::Cols>) -> Result<Self, tokio_postgres::Error>;
}

impl<Cols> Deref for Row<Cols> {
    type Target = tokio_postgres::Row;

    fn deref(&self) -> &Self::Target {
        &self.row
    }
}

impl<Cols> From<tokio_postgres::Row> for Row<Cols> {
    fn from(row: tokio_postgres::Row) -> Self {
        Self {
            row,
            marker: PhantomData,
        }
    }
}
