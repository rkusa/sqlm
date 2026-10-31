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

/// Marks a value that may be `NULL`: on the query side a column that refers to a table column
/// without `NOT NULL` constraint, on the struct side an [`Option`] field.
pub struct Nullable<T>(PhantomData<T>);

/// Implemented by `T` and [`Nullable<T>`], i.e. the column types an `Option<T>` can be read from.
pub trait MaybeNullable<T> {}

impl<T> MaybeNullable<T> for T {}
impl<T> MaybeNullable<T> for Nullable<T> {}

/// Implemented by a struct's column list ([`FromRow::Cols`]) if the struct can be read from a
/// query returning the columns `Cols`.
///
/// [`FromRow::Cols`]: crate::FromRow::Cols
#[diagnostic::on_unimplemented(
    message = "the struct's columns `{Self}` do not match the query's columns `{Cols}`",
    label = "the struct's fields do not match the columns returned by the query",
    note = "fields and columns are compared by name (sorted) and type; the number of fields must \
            equal the number of columns"
)]
pub trait Compatible<Cols> {}

/// Implemented by a struct's column if it can be read from the query column `Col`: name and type
/// must be equal, and a column wrapped in [`Nullable`] requires an [`Option`] field.
#[diagnostic::on_unimplemented(
    message = "the field `{Self}` cannot be read from the column `{Col}`",
    label = "field and column differ in name, type, or nullability",
    note = "a column wrapped in `Nullable` may be `NULL` and must be read into an `Option`"
)]
pub trait ColumnCompatible<Col> {}

macro_rules! impl_column_compatible {
    ($name:ty) => {
        // Same name and type (also covers an `Option` field reading a `Nullable` column).
        impl<T, const NAME: $name> ColumnCompatible<StructColumn<T, NAME>>
            for StructColumn<T, NAME>
        {
        }

        // An `Option` field can also read a column that is never `NULL` (or of unknown
        // nullability).
        impl<T, const NAME: $name> ColumnCompatible<StructColumn<T, NAME>>
            for StructColumn<Nullable<T>, NAME>
        {
        }
    };
}

#[cfg(not(nightly_column_names))]
impl_column_compatible!(usize);
#[cfg(nightly_column_names)]
impl_column_compatible!(&'static str);

macro_rules! impl_compatible {
    ($($S:ident $Q:ident),*) => {
        impl<$($S, $Q,)*> Compatible<($($Q,)*)> for ($($S,)*)
        where
            $($S: ColumnCompatible<$Q>,)*
        {
        }
    };
}

// Implements `Compatible` for all tuple sizes from the given one down to zero.
macro_rules! impl_compatible_tuples {
    () => {
        impl_compatible!();
    };
    ($S:ident $Q:ident $(, $Ss:ident $Qs:ident)*) => {
        impl_compatible!($S $Q $(, $Ss $Qs)*);
        impl_compatible_tuples!($($Ss $Qs),*);
    };
}

impl_compatible_tuples!(
    S1 Q1, S2 Q2, S3 Q3, S4 Q4, S5 Q5, S6 Q6, S7 Q7, S8 Q8, S9 Q9, S10 Q10, S11 Q11, S12 Q12,
    S13 Q13, S14 Q14, S15 Q15, S16 Q16, S17 Q17, S18 Q18, S19 Q19, S20 Q20, S21 Q21, S22 Q22,
    S23 Q23, S24 Q24, S25 Q25, S26 Q26, S27 Q27, S28 Q28, S29 Q29, S30 Q30, S31 Q31, S32 Q32,
    S33 Q33, S34 Q34, S35 Q35, S36 Q36, S37 Q37, S38 Q38, S39 Q39, S40 Q40, S41 Q41, S42 Q42,
    S43 Q43, S44 Q44, S45 Q45, S46 Q46, S47 Q47, S48 Q48, S49 Q49, S50 Q50, S51 Q51, S52 Q52,
    S53 Q53, S54 Q54, S55 Q55, S56 Q56, S57 Q57, S58 Q58, S59 Q59, S60 Q60, S61 Q61, S62 Q62,
    S63 Q63, S64 Q64, S65 Q65, S66 Q66, S67 Q67, S68 Q68, S69 Q69, S70 Q70, S71 Q71, S72 Q72,
    S73 Q73, S74 Q74, S75 Q75, S76 Q76, S77 Q77, S78 Q78, S79 Q79, S80 Q80, S81 Q81, S82 Q82,
    S83 Q83, S84 Q84, S85 Q85, S86 Q86, S87 Q87, S88 Q88, S89 Q89, S90 Q90, S91 Q91, S92 Q92,
    S93 Q93, S94 Q94, S95 Q95, S96 Q96, S97 Q97, S98 Q98, S99 Q99, S100 Q100, S101 Q101,
    S102 Q102, S103 Q103, S104 Q104, S105 Q105, S106 Q106, S107 Q107, S108 Q108, S109 Q109,
    S110 Q110, S111 Q111, S112 Q112, S113 Q113, S114 Q114, S115 Q115, S116 Q116, S117 Q117,
    S118 Q118, S119 Q119, S120 Q120, S121 Q121, S122 Q122, S123 Q123, S124 Q124, S125 Q125,
    S126 Q126, S127 Q127, S128 Q128
);
