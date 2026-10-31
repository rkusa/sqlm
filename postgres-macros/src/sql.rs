use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};

use postgres::config::SslMode;
use postgres::{Client, Column, Config};
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Expr, LitStr, Type, parse_macro_input, parse_quote};

use crate::const_name;
use crate::parser::{self, Argument, Token};

pub fn sql(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as Input);
    // dbg!(&input);

    let mut unnamed_arguments = Vec::new();
    let mut named_arguments = HashMap::new();
    let mut variable_arguments = HashMap::new();
    if let Some(arguments) = input.arguments {
        for expr in arguments.arguments {
            if let Expr::Assign(expr_assign) = expr {
                named_arguments.insert(
                    expr_assign.left.into_token_stream().to_string(),
                    Parameter {
                        expr: expr_assign.right,
                        index: None,
                    },
                );
            } else {
                if !named_arguments.is_empty() {
                    return syn::Error::new(
                        expr.span(),
                        "positional arguments cannot follow named arguments",
                    )
                    .into_compile_error()
                    .into();
                }

                unnamed_arguments.push(Parameter {
                    expr: Box::new(expr),
                    index: None,
                });
            }
        }
    }

    let mut next_arg = 0;
    let query = input.query.value();
    let mut result = String::with_capacity(query.len());
    let mut parameters = Vec::new();

    let tokens = match parser::parse(&query) {
        Ok(tokens) => tokens,
        Err(err) => {
            return syn::Error::new(input.query.span(), err)
                .into_compile_error()
                .into();
        }
    };

    for token in tokens {
        let index = match token {
            Token::EscapedCurlyStart => {
                result.push('{');
                continue;
            }
            Token::EscapedCurlyEnd => {
                result.push('}');
                continue;
            }
            Token::Text(text) => {
                result.push_str(text);
                continue;
            }
            Token::Argument(Argument::Next) => {
                let Some(param) = unnamed_arguments.get_mut(next_arg) else {
                    return syn::Error::new(
                        input.query.span(),
                        format!("missing argument for position {next_arg}"),
                    )
                    .into_compile_error()
                    .into();
                };
                next_arg += 1;
                if let Some(index) = param.index {
                    index
                } else {
                    parameters.push(param.expr.to_token_stream());
                    let index = parameters.len();
                    param.index = Some(index);
                    index
                }
            }
            Token::Argument(Argument::Positional(ix)) => {
                let Some(param) = unnamed_arguments.get_mut(ix) else {
                    return syn::Error::new(
                        input.query.span(),
                        format!("missing argument for index {ix}"),
                    )
                    .into_compile_error()
                    .into();
                };
                if let Some(index) = param.index {
                    index
                } else {
                    parameters.push(param.expr.to_token_stream());
                    let index = parameters.len();
                    param.index = Some(index);
                    index
                }
            }
            Token::Argument(Argument::Named(ident)) => {
                if let Some(param) = named_arguments.get_mut(ident) {
                    if let Some(index) = param.index {
                        index
                    } else {
                        parameters.push(param.expr.to_token_stream());
                        let index = parameters.len();
                        param.index = Some(index);
                        index
                    }
                } else {
                    match variable_arguments.entry(ident) {
                        Entry::Occupied(e) => *e.get(),
                        Entry::Vacant(e) => {
                            let ident = format_ident!("{}", ident);
                            parameters.push(ident.to_token_stream());
                            let index = parameters.len();
                            e.insert(index);
                            index
                        }
                    }
                }
            }
        };

        write!(result, "${}", index).unwrap();
    }

    for arg in unnamed_arguments
        .into_iter()
        .chain(named_arguments.into_values())
    {
        if arg.index.is_none() {
            return syn::Error::new(arg.expr.span(), "argument never used")
                .into_compile_error()
                .into();
        }
    }

    let info = match inspect(&result) {
        Ok(info) => info,
        Err(err) => {
            return syn::Error::new(input.query.span(), err)
                .into_compile_error()
                .into();
        }
    };

    let mut typed_parameters = Vec::with_capacity(parameters.len());
    for (ty, param) in info.params.iter().zip(parameters) {
        if let Some((is_array, variants)) = enum_type(ty) {
            let enum_ty = enum_tokens(&variants);
            if is_array {
                typed_parameters.push(quote! {
                    ::sqlm_postgres::internal::assert_type_slice::<_, #enum_ty>(&#param)
                });
            } else {
                typed_parameters.push(quote! {
                    ::sqlm_postgres::internal::assert_type::<_, #enum_ty>(&#param)
                });
            }
            continue;
        }

        let Some((ty, _)) = postgres_to_rust_type(ty) else {
            return syn::Error::new(
                input.query.span(),
                format!("unsupported postgres type: {ty:?}"),
            )
            .into_compile_error()
            .into();
        };

        typed_parameters.push(quote! {
            ::sqlm_postgres::internal::assert_type::<_, #ty>(&(#param))
        });
    }

    let cols = match info.columns.as_slice() {
        [] => quote!(()),
        // Consider the result to be a literal
        [column] => {
            let ty = &column.ty;
            if let Some((is_array, variants)) = enum_type(ty) {
                let enum_ty = enum_tokens(&variants);
                if is_array {
                    quote!(::sqlm_postgres::types::Array<Vec<#enum_ty>>)
                } else {
                    let ty = nullable(enum_ty, column.nullable);
                    quote!(::sqlm_postgres::types::Primitive<#ty>)
                }
            } else if let Some((ty, is_array)) = postgres_to_rust_type(ty) {
                if is_array {
                    quote!(::sqlm_postgres::types::Array<#ty>)
                } else {
                    let ty = nullable(ty, column.nullable);
                    quote!(::sqlm_postgres::types::Primitive<#ty>)
                }
            } else {
                return syn::Error::new(
                    input.query.span(),
                    format!("unsupported postgres type: {ty:?}"),
                )
                .into_compile_error()
                .into();
            }
        }
        columns => {
            let mut columns = columns.iter().collect::<Vec<_>>();
            columns.sort_by_key(|c| c.name.as_str());

            let mut struct_columns: Vec<Type> = Vec::with_capacity(columns.len());
            for column in columns {
                let ty = &column.ty;
                let name = const_name(&column.name);
                let ty = if let Some((is_array, variants)) = enum_type(ty) {
                    let enum_ty = enum_tokens(&variants);
                    if is_array {
                        quote!(Vec<#enum_ty>)
                    } else {
                        enum_ty
                    }
                } else if let Some((ty, _)) = postgres_to_rust_type(ty) {
                    ty
                } else {
                    return syn::Error::new(
                        input.query.span(),
                        format!("unsupported postgres type: {ty:?}"),
                    )
                    .into_compile_error()
                    .into();
                };
                let ty = nullable(ty, column.nullable);
                struct_columns.push(parse_quote!(::sqlm_postgres::types::StructColumn<#ty, #name>));
            }

            quote!(::sqlm_postgres::types::Struct<(#(#struct_columns,)*)>)
        }
    };

    // For queries without columns, the result type is fixed to `()`.
    let result_ty = if info.columns.is_empty() {
        quote!(())
    } else {
        quote!(_)
    };
    quote! {
        ::sqlm_postgres::Sql::<'_, #cols, #result_ty> {
            query: #result,
            parameters: &[#(&(#typed_parameters),)*],
            marker: ::std::marker::PhantomData,
        }
    }
    .into()
}

/// Wraps `ty` in `Nullable` if the column may be `NULL`.
fn nullable(ty: proc_macro2::TokenStream, nullable: bool) -> proc_macro2::TokenStream {
    if nullable {
        quote!(::sqlm_postgres::types::Nullable<#ty>)
    } else {
        ty
    }
}

/// The type-level representation of a Postgres enum with the given (sorted) variants.
fn enum_tokens(variants: &[String]) -> proc_macro2::TokenStream {
    let variants = variants.iter().map(|variant| {
        let name = const_name(variant);
        quote!(::sqlm_postgres::types::EnumVariant<#name>)
    });
    quote!(::sqlm_postgres::types::Enum<(#(#variants,)*)>)
}

/// Maps a Postgres type to the Rust type it is read into, plus whether it is an array.
fn postgres_to_rust_type(ty: &postgres::types::Type) -> Option<(proc_macro2::TokenStream, bool)> {
    use postgres::types::{FromSql, Kind};

    if let Kind::Array(ty) = ty.kind() {
        // reject nested array
        if matches!(ty.kind(), Kind::Array(_)) {
            return None;
        }

        return postgres_to_rust_type(ty).map(|(ty, _)| (quote!(Vec<#ty>), true));
    }

    let ty = match ty {
        ty if <String as FromSql>::accepts(ty) => quote!(String),
        ty if <i64 as FromSql>::accepts(ty) => quote!(i64),
        ty if <i32 as FromSql>::accepts(ty) => quote!(i32),
        ty if <f64 as FromSql>::accepts(ty) => quote!(f64),
        ty if <f32 as FromSql>::accepts(ty) => quote!(f32),
        ty if <bool as FromSql>::accepts(ty) => quote!(bool),
        ty if <Vec<u8> as FromSql>::accepts(ty) => quote!(::sqlm_postgres::types::Bytea),
        #[cfg(feature = "json")]
        ty if <::serde_json::Value as FromSql>::accepts(ty) => quote!(::serde_json::Value),
        #[cfg(feature = "time")]
        ty if <::time::Date as FromSql>::accepts(ty) => quote!(::time::Date),
        #[cfg(feature = "time")]
        ty if <::time::OffsetDateTime as FromSql>::accepts(ty) => quote!(::time::OffsetDateTime),
        #[cfg(feature = "uuid")]
        ty if <::uuid::Uuid as FromSql>::accepts(ty) => quote!(::uuid::Uuid),
        #[cfg(feature = "pgvector")]
        ty if <::pgvector::Vector as FromSql>::accepts(ty) => quote!(::pgvector::Vector),
        _ => return None,
    };
    Some((ty, false))
}

fn enum_type(ty: &postgres::types::Type) -> Option<(bool, Vec<String>)> {
    use postgres::types::Kind;
    let mut data = match ty.kind() {
        Kind::Enum(variants) => Some((false, variants.clone())),
        Kind::Array(ty) => {
            if let Kind::Enum(variants) = ty.kind() {
                Some((true, variants.clone()))
            } else {
                None
            }
        }
        _ => None,
    };
    if let Some((_, variants)) = &mut data {
        variants.sort();
    }
    data
}

struct Parameter {
    expr: Box<Expr>,
    index: Option<usize>,
}

#[derive(Debug)]
struct Input {
    query: LitStr,
    arguments: Option<Arguments>,
}

#[derive(Debug)]
struct Arguments {
    #[allow(unused)]
    comma: Comma,
    arguments: Punctuated<Expr, Comma>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            query: input.parse()?,
            arguments: input
                .peek(Comma)
                .then(|| {
                    Ok::<_, syn::Error>(Arguments {
                        comma: Comma::parse(input)?,
                        // TODO: no unnamed after named
                        arguments: Punctuated::<Expr, Comma>::parse_terminated(input)?,
                    })
                })
                .transpose()?,
        })
    }
}

/// The database connection shared by all `sql!` invocations within one compiler process, so that
/// not every single query has to establish a new connection.
static CLIENT: Mutex<Option<CachedClient>> = Mutex::new(None);

struct CachedClient {
    database_url: String,
    client: Client,
}

struct QueryInfo {
    params: Vec<postgres::types::Type>,
    columns: Vec<ColumnInfo>,
}

struct ColumnInfo {
    name: String,
    ty: postgres::types::Type,
    /// Whether the column is known to possibly be `NULL`, i.e. it refers to a table column without
    /// `NOT NULL` constraint. `false` also covers columns of unknown nullability (expressions,
    /// aggregates, ...).
    nullable: bool,
}

/// Prepares `query` on the cached connection (connecting first if necessary) to retrieve its
/// parameter and column types. Errors are returned as messages ready to be reported at the query.
fn inspect(query: &str) -> Result<QueryInfo, String> {
    let Ok(database_url) = dotenvy::var("DATABASE_URL") else {
        return Err(
            "compile-time query checks require DATABASE_URL environment variable to be defined"
                .to_string(),
        );
    };

    let mut cached = CLIENT.lock().unwrap_or_else(PoisonError::into_inner);
    // Drop a cached connection that got closed or that belongs to a different database.
    if cached
        .as_ref()
        .is_some_and(|c| c.database_url != database_url || c.client.is_closed())
    {
        *cached = None;
    }
    if cached.is_none() {
        *cached = Some(CachedClient {
            client: connect(&database_url)?,
            database_url,
        });
    }
    let cached = cached.as_mut().unwrap();

    match inspect_with(&mut cached.client, query) {
        Err(err) if err.is_closed() => {
            // The connection died since it was last used, e.g. due to a server-side timeout.
            cached.client = connect(&cached.database_url)?;
            inspect_with(&mut cached.client, query).map_err(query_error)
        }
        result => result.map_err(query_error),
    }
}

fn inspect_with(client: &mut Client, query: &str) -> Result<QueryInfo, postgres::Error> {
    let stmt = client.prepare(query)?;
    let nullable = nullable_columns(client, stmt.columns())?;
    Ok(QueryInfo {
        params: stmt.params().to_vec(),
        columns: stmt
            .columns()
            .iter()
            .zip(nullable)
            .map(|(column, nullable)| ColumnInfo {
                name: column.name().to_string(),
                ty: column.type_().clone(),
                nullable,
            })
            .collect(),
    })
}

/// Determines for each column whether it is known to possibly be `NULL`. This is only possible for
/// columns that directly refer to a table column (via its `NOT NULL` constraint); all other columns
/// are reported as `false`.
fn nullable_columns(client: &mut Client, columns: &[Column]) -> Result<Vec<bool>, postgres::Error> {
    let refs = columns
        .iter()
        .filter_map(|c| Some((c.table_oid()?, c.column_id()?)))
        .collect::<Vec<_>>();
    if refs.is_empty() {
        return Ok(vec![false; columns.len()]);
    }

    let (rels, nums): (Vec<u32>, Vec<i16>) = refs.iter().copied().unzip();
    let not_null = client
        .query(
            "SELECT attrelid, attnum, attnotnull FROM pg_attribute \
             WHERE attrelid = ANY($1) AND attnum = ANY($2)",
            &[&rels, &nums],
        )?
        .into_iter()
        .map(|row| {
            (
                (row.get::<_, u32>(0), row.get::<_, i16>(1)),
                row.get::<_, bool>(2),
            )
        })
        .collect::<HashMap<_, _>>();

    Ok(columns
        .iter()
        .map(|c| match (c.table_oid(), c.column_id()) {
            (Some(oid), Some(num)) => not_null.get(&(oid, num)) == Some(&false),
            _ => false,
        })
        .collect())
}

fn query_error(err: postgres::Error) -> String {
    if let Some(db_error) = err.as_db_error() {
        format!("query failed: {db_error}")
    } else {
        format!("query failed: {err}")
    }
}

fn connect(database_url: &str) -> Result<Client, String> {
    let mut config = Config::from_str(database_url)
        .map_err(|err| format!("failed to parse connection config from DATABASE_URL: {err}"))?;
    if config.get_application_name().is_none() {
        config.application_name(env!("CARGO_PKG_NAME"));
    }

    // TODO: take all possible SSL variants into account, see e.g.
    // https://github.com/jbg/tokio-postgres-rustls/issues/11
    let client = match config.get_ssl_mode() {
        SslMode::Disable => config.connect(postgres::NoTls),
        _ => config.connect(tokio_postgres_rustls::MakeRustlsConnect::new(tls_config())),
    };
    client.map_err(|err| format!("failed to connect to postgres (using DATABASE_URL): {err}"))
}

/// A TLS config that encrypts the connection but does not verify the server certificate. Uses the
/// process-wide default crypto provider if one is installed and falls back to `ring` otherwise.
fn tls_config() -> rustls::ClientConfig {
    let crypto_provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));
    rustls::ClientConfig::builder_with_provider(Arc::clone(&crypto_provider))
        .with_safe_default_protocol_versions()
        .expect("crypto provider supports the default TLS protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoServerCertVerify { crypto_provider }))
        .with_no_client_auth()
}

#[derive(Debug)]
struct NoServerCertVerify {
    crypto_provider: Arc<rustls::crypto::CryptoProvider>,
}

impl rustls::client::danger::ServerCertVerifier for NoServerCertVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.crypto_provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
