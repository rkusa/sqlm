use std::marker::PhantomData;

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
