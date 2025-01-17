use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use tokio_postgres::types::{FromSqlOwned, ToSql};

use crate::{Error, Sql};

/// A trait used to which Rust type a Postgres type is read into.
///
/// # Example
///
/// This can be useful to implement manually when e.g. reading a Postgres string column into an
/// enum.
///
/// ```
/// #[derive(Debug, Default, Clone, Copy)]
/// pub enum Role {
///     #[default]
///     User,
///     Admin,
/// }
///
/// impl sqlm_postgres::SqlType for Role {
///     type Type = String;
/// }
///
/// impl Role {
///     pub fn as_str(&self) -> &'static str {
///         match self {
///             Self::User => "user",
///             Self::Admin => "admin",
///         }
///     }
/// }
///
/// impl<'a> sqlm_postgres::FromSql<'a> for Role {
///     fn from_sql(
///         ty: &postgres_types::Type,
///         raw: &'a [u8],
///     ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
///         use std::str::FromStr;
///         Ok(Role::from_str(<&str>::from_sql(ty, raw)?)?)
///     }
///
///     fn accepts(ty: &postgres_types::Type) -> bool {
///         <&str as sqlm_postgres::FromSql<'_>>::accepts(ty)
///     }
/// }
///
/// impl sqlm_postgres::ToSql for Role {
///     fn to_sql(
///         &self,
///         ty: &postgres_types::Type,
///         out: &mut bytes::BytesMut,
///     ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>>
///     where
///         Self: Sized,
///     {
///         <&str as sqlm_postgres::ToSql>::to_sql(&self.as_str(), ty, out)
///     }
///
///     fn accepts(ty: &postgres_types::Type) -> bool
///     where
///         Self: Sized,
///     {
///         <&str as sqlm_postgres::ToSql>::accepts(ty)
///     }
///
///     fn to_sql_checked(
///         &self,
///         ty: &postgres_types::Type,
///         out: &mut bytes::BytesMut,
///     ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
///         <&str as sqlm_postgres::ToSql>::to_sql_checked(&self.as_str(), ty, out)
///     }
/// }
///
/// impl std::str::FromStr for Role {
///     type Err = UnknownRole;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         match s {
///             "user" => Ok(Role::User),
///             "admin" => Ok(Role::Admin),
///             s => Err(UnknownRole(s.to_string())),
///         }
///     }
/// }
///
/// #[derive(Debug)]
/// pub struct UnknownRole(String);
///
/// impl std::fmt::Display for UnknownRole {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "Unknown bevel lhs `{}`", self.0)
///     }
/// }
///
/// impl std::error::Error for UnknownRole {}
/// ```
pub trait SqlType {
    type Type;

    fn query_literal<'a>(
        sql: &'a Sql<'a, Primitive<Self::Type>, Self>,
        conn: impl super::Connection + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>
    where
        Self: FromSqlOwned + ToSql + Send + Sync,
        Self::Type: Send + Sync,
    {
        Box::pin(async move {
            let row = conn.query_one(sql.query, sql.parameters).await?;
            Ok(row.try_get(0)?)
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytea(pub Vec<u8>);

#[cfg(not(nightly_column_names))]
pub struct StructColumn<T, const NAME: usize>(PhantomData<T>);
#[cfg(nightly_column_names)]
pub struct StructColumn<T, const NAME: &'static str>(PhantomData<T>);

pub struct Struct<T>(PhantomData<T>);

pub struct Primitive<T>(PhantomData<T>);

pub struct Array<T>(PhantomData<T>);

pub struct Enum<T>(PhantomData<T>);

#[cfg(not(nightly_column_names))]
pub struct EnumVariant<const NAME: usize>(());
#[cfg(nightly_column_names)]
pub struct EnumVariant<const NAME: &'static str>(());

macro_rules! impl_type {
    ($ty:path) => {
        impl SqlType for $ty {
            type Type = Self;
        }

        impl<'a> SqlType for &'a $ty {
            type Type = $ty;
        }
    };
}

impl_type!(i32);
impl_type!(i64);
impl_type!(f32);
impl_type!(f64);
impl_type!(bool);
impl_type!(String);
#[cfg(feature = "json")]
impl_type!(serde_json::Value);
#[cfg(feature = "time")]
impl_type!(time::OffsetDateTime);
#[cfg(feature = "time")]
impl_type!(time::Date);
#[cfg(feature = "uuid")]
impl_type!(uuid::Uuid);
#[cfg(feature = "pgvector")]
impl_type!(pgvector::Vector);

impl SqlType for &str {
    type Type = String;
}
