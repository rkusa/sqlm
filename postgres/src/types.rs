use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use tokio_postgres::types::{FromSqlOwned, ToSql};

use crate::{Error, Sql};

pub trait SqlType {
    type Type;

    fn query_literal<'a>(
        sql: &'a Sql<'a, Primitive<Self::Type>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>
    where
        Self: FromSqlOwned + ToSql + Send + Sync,
        Self::Type: Send + Sync,
    {
        Box::pin(async move {
            let row = sql.query_one().await?;
            Ok(row.try_get(0)?)
        })
    }
}

#[cfg(not(nightly_column_names))]
pub struct StructColumn<T, const NAME: usize>(PhantomData<T>);
#[cfg(nightly_column_names)]
pub struct StructColumn<T, const NAME: &'static str>(PhantomData<T>);

pub struct Struct<T>(PhantomData<T>);

pub struct Primitive<T>(PhantomData<T>);

pub struct Enum<T>(PhantomData<T>);

#[cfg(not(nightly_column_names))]
pub struct EnumVariant<const NAME: usize>(());
#[cfg(nightly_column_names)]
pub struct EnumVariant<const NAME: &'static str>(());

impl<T> SqlType for Option<T>
where
    T: SqlType,
{
    type Type = T::Type;

    fn query_literal<'a>(
        sql: &'a Sql<'a, Primitive<Self::Type>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>
    where
        Self: FromSqlOwned + ToSql + Send + Sync,
        Self::Type: Send + Sync,
    {
        Box::pin(async move {
            let row = sql.query_opt().await?;
            match row {
                Some(row) => Ok(row.try_get::<'_, _, Option<T>>(0)?),
                None => Ok(None),
            }
        })
    }
}

impl<T> SqlType for Vec<T>
where
    T: SqlType,
{
    type Type = Vec<T::Type>;

    fn query_literal<'a>(
        sql: &'a Sql<'a, Primitive<Self::Type>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>
    where
        Self: FromSqlOwned + ToSql + Send + Sync,
        Self::Type: Send + Sync,
    {
        Box::pin(async move {
            let row = sql.query_one().await?;
            Ok(row.try_get(0)?)
        })
    }
}

// TODO: add test for that
impl SqlType for Vec<u8> {
    type Type = Vec<u8>;

    fn query_literal<'a>(
        sql: &'a Sql<'a, Primitive<Self::Type>, Self>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>
    where
        Self: FromSqlOwned + ToSql + Send + Sync,
        Self::Type: Send + Sync,
    {
        Box::pin(async move {
            let row = sql.query_one().await?;
            Ok(row.try_get(0)?)
        })
    }
}

macro_rules! impl_type {
    ($ty:path) => {
        impl SqlType for $ty {
            type Type = Self;
        }
    };
}

impl_type!(i32);
impl_type!(i64);
impl_type!(bool);
impl_type!(String);
#[cfg(feature = "json")]
impl_type!(serde_json::Value);
#[cfg(feature = "time")]
impl_type!(time::OffsetDateTime);
#[cfg(feature = "uuid")]
impl_type!(uuid::Uuid);

impl<'a> SqlType for &'a str {
    type Type = String;
}
