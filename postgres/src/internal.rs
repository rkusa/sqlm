use std::marker::PhantomData;

use crate::SqlType;
use crate::types::Bytea;

pub struct Valid<'a, B: 'a + ?Sized, O = B>(PhantomData<(&'a B, O)>);

impl<'a, T> From<&'a T> for Valid<'a, T, T> {
    fn from(_: &'a T) -> Self {
        Self(PhantomData)
    }
}

impl<T> From<T> for Valid<'_, T, T> {
    fn from(_: T) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<Option<&'a T>> for Valid<'a, T, T> {
    fn from(_: Option<&'a T>) -> Self {
        Self(PhantomData)
    }
}

impl<T> From<Option<T>> for Valid<'_, T, T> {
    fn from(_: Option<T>) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<&'a str> for Valid<'a, str, String> {
    fn from(_: &'a str) -> Self {
        Self(PhantomData)
    }
}

impl From<String> for Valid<'_, str, String> {
    fn from(_: String) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<Option<&'a str>> for Valid<'a, str, String> {
    fn from(_: Option<&'a str>) -> Self {
        Self(PhantomData)
    }
}

impl From<Option<String>> for Valid<'_, str, String> {
    fn from(_: Option<String>) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<&'a [T]> for Valid<'a, [T], Vec<T>> {
    fn from(_: &'a [T]) -> Self {
        Self(PhantomData)
    }
}

impl<T> From<T> for Valid<'_, [T], Vec<T>> {
    fn from(_: T) -> Self {
        Self(PhantomData)
    }
}

impl<T> From<Vec<T>> for Valid<'_, [T], Vec<T>> {
    fn from(_: Vec<T>) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<Option<&'a [T]>> for Valid<'a, [T], Vec<T>> {
    fn from(_: Option<&'a [T]>) -> Self {
        Self(PhantomData)
    }
}

impl<T> From<Option<Vec<T>>> for Valid<'_, [T], Vec<T>> {
    fn from(_: Option<Vec<T>>) -> Self {
        Self(PhantomData)
    }
}

impl From<i32> for Valid<'_, i64> {
    fn from(_: i32) -> Self {
        Self(PhantomData)
    }
}

impl From<Vec<u8>> for Valid<'_, Bytea> {
    fn from(_: Vec<u8>) -> Self {
        Self(PhantomData)
    }
}

impl From<Vec<Vec<u8>>> for Valid<'_, [Bytea], Vec<Bytea>> {
    fn from(_: Vec<Vec<u8>>) -> Self {
        Self(PhantomData)
    }
}

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
    type SqlType = Self;
}
