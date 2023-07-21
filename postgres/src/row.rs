use std::marker::PhantomData;
use std::ops::Deref;

pub struct Row<Cols> {
    row: tokio_postgres::Row,
    marker: PhantomData<Cols>,
}

pub trait FromRow<Cols>: Sized {
    fn from_row(row: Row<Cols>) -> Result<Self, tokio_postgres::Error>;
}

#[cfg(not(nightly_column_names))]
pub trait HasColumn<Type, const NAME: usize> {}
#[cfg(nightly_column_names)]
pub trait HasColumn<Type, const NAME: &'static str> {}

#[cfg(not(nightly_column_names))]
pub trait HasVariant<const N: usize, const NAME: usize> {}
#[cfg(nightly_column_names)]
pub trait HasVariant<const N: usize, const NAME: &'static str> {}

#[cfg(feature = "comptime")]
#[derive(Default)]
pub struct Struct<T>(PhantomData<T>);

#[cfg(feature = "comptime")]
#[derive(Default)]
pub struct Literal<T>(PhantomData<T>);

#[cfg(feature = "comptime")]
#[derive(Default)]
pub struct EnumArray<T>(PhantomData<T>);

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
