use std::marker::PhantomData;
use std::ops::Deref;

/// A row of data returned from Postgres.
pub struct Row<Cols> {
    row: tokio_postgres::Row,
    marker: PhantomData<Cols>,
}

/// A trait for types that can be created from a [`Row`] (a postgres row containing columns as
/// constraint by `Cols`).
///
/// This is usually derived via [`FromRow`] and not implemented manually.
///
/// [`FromRow`]: `derive@crate::FromRow`
pub trait FromRow<Cols>: Sized {
    fn from_row(row: Row<Cols>) -> Result<Self, tokio_postgres::Error>;
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
