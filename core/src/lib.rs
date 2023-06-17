use std::fmt;
use std::marker::PhantomData;

pub trait ToSql: fmt::Display {}

pub struct Sql<'a, T> {
    pub query: &'static str,
    pub parameters: &'a [&'a dyn ToSql],
    pub marker: PhantomData<T>,
}

impl ToSql for usize {}
impl ToSql for f32 {}
impl ToSql for f64 {}
impl<'a> ToSql for &'a str {}
