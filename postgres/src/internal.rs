use crate::SqlType;
use crate::types::Bytea;

/// Asserts at compile time that `t` can be passed as a parameter of the Postgres type that maps to
/// the Rust type `S`.
#[inline]
pub const fn assert_type<T, S>(t: &T) -> &T
where
    T: AsSqlType<SqlType = S>,
{
    t
}

/// Like [`assert_type`], but for each element of a slice.
#[inline]
pub const fn assert_type_slice<T, S>(t: &[T]) -> &[T]
where
    T: AsSqlType<SqlType = S>,
{
    t
}

/// The Rust type a parameter of this type maps to on the Postgres side. Unlike [`SqlType`], this
/// is also implemented for `Option`s and `Vec`s of such types.
pub trait AsSqlType {
    type SqlType;
}

impl<T> AsSqlType for T
where
    T: SqlType,
{
    type SqlType = T::Type;
}

impl<T> AsSqlType for Option<T>
where
    T: SqlType,
{
    type SqlType = T::Type;
}

impl<T> AsSqlType for Vec<T>
where
    T: SqlType,
{
    type SqlType = Vec<T::Type>;
}

impl<T> AsSqlType for Option<Vec<T>>
where
    T: SqlType,
{
    type SqlType = Vec<T::Type>;
}

// BYTEA
impl AsSqlType for Vec<u8> {
    type SqlType = Bytea;
}
impl AsSqlType for Option<Vec<u8>> {
    type SqlType = Bytea;
}
impl AsSqlType for &[u8] {
    type SqlType = Bytea;
}
impl AsSqlType for Option<&[u8]> {
    type SqlType = Bytea;
}
impl AsSqlType for Vec<Vec<u8>> {
    type SqlType = Vec<Bytea>;
}
