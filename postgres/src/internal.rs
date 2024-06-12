use std::marker::PhantomData;

use crate::types::Bytea;
use crate::SqlType;

pub struct Valid<'a, B: 'a + ?Sized, O = B>(PhantomData<(&'a B, O)>);

impl<'a, T> From<&'a T> for Valid<'a, T, T> {
    fn from(_: &'a T) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<T> for Valid<'a, T, T> {
    fn from(_: T) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<Option<&'a T>> for Valid<'a, T, T> {
    fn from(_: Option<&'a T>) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<Option<T>> for Valid<'a, T, T> {
    fn from(_: Option<T>) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<&'a str> for Valid<'a, str, String> {
    fn from(_: &'a str) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<String> for Valid<'a, str, String> {
    fn from(_: String) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<Option<&'a str>> for Valid<'a, str, String> {
    fn from(_: Option<&'a str>) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<Option<String>> for Valid<'a, str, String> {
    fn from(_: Option<String>) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<&'a [T]> for Valid<'a, [T], Vec<T>> {
    fn from(_: &'a [T]) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<T> for Valid<'a, [T], Vec<T>> {
    fn from(_: T) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<Vec<T>> for Valid<'a, [T], Vec<T>> {
    fn from(_: Vec<T>) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<Option<&'a [T]>> for Valid<'a, [T], Vec<T>> {
    fn from(_: Option<&'a [T]>) -> Self {
        Self(PhantomData)
    }
}

impl<'a, T> From<Option<Vec<T>>> for Valid<'a, [T], Vec<T>> {
    fn from(_: Option<Vec<T>>) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<i32> for Valid<'a, i64> {
    fn from(_: i32) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<Vec<u8>> for Valid<'a, Bytea> {
    fn from(_: Vec<u8>) -> Self {
        Self(PhantomData)
    }
}

impl<'a> From<Vec<Vec<u8>>> for Valid<'a, [Bytea], Vec<Bytea>> {
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

// BYTEA
impl AsSqlType for Vec<u8> {
    type SqlType = Bytea;
}
impl<'a> AsSqlType for &'a [u8] {
    type SqlType = Bytea;
}

impl AsSqlType for Vec<Vec<u8>> {
    type SqlType = Self;
}
