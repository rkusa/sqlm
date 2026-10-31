use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};

use postgres::config::SslMode;
use postgres::{Client, Config, Statement};
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

    let stmt = match prepare(&result) {
        Ok(stmt) => stmt,
        Err(err) => {
            return syn::Error::new(input.query.span(), err)
                .into_compile_error()
                .into();
        }
    };

    let mut typed_parameters = Vec::with_capacity(parameters.len());
    for (ty, param) in stmt.params().iter().zip(parameters) {
        if let Some((is_array, variants)) = enum_type(ty) {
            let mut enum_variants: Vec<Type> = Vec::with_capacity(variants.len());
            for variant in variants {
                let name = const_name(&variant);
                enum_variants.push(parse_quote!(::sqlm_postgres::types::EnumVariant<#name>));
            }

            let enum_struct = quote! { ::sqlm_postgres::types::Enum<(#(#enum_variants,)*)> };
            if is_array {
                typed_parameters.push(quote! {
                    ::sqlm_postgres::internal::assert_type_enum_slice::<_, #enum_struct>(&#param)
                });
            } else {
                typed_parameters.push(quote! {
                    ::sqlm_postgres::internal::assert_type_enum::<_, #enum_struct>(&#param)
                });
            }
            continue;
        }

        let Some((ty_owned, ty_borrowed, _)) = postgres_to_rust_type(ty) else {
            return syn::Error::new(
                input.query.span(),
                format!("unsupported postgres type: {ty:?}"),
            )
            .into_compile_error()
            .into();
        };

        typed_parameters.push(quote! {
            ::sqlm_postgres::internal::assert_type::<_, _, #ty_borrowed, #ty_owned>(&(#param))
        });
    }

    let col_count = stmt.columns().len();
    if col_count == 0 {
        return quote! {
            ::sqlm_postgres::Sql::<'_, (), ()> {
                query: #result,
                parameters: &[#(&(#typed_parameters),)*],
                marker: ::std::marker::PhantomData,
            }
        }
        .into();
    } else if col_count == 1 {
        // Consider the result to be a literal
        let ty = stmt.columns()[0].type_();
        if let Some((is_array, variants)) = enum_type(ty) {
            let mut enum_variants: Vec<Type> = Vec::with_capacity(variants.len());
            for variant in variants {
                let name = const_name(&variant);
                enum_variants.push(parse_quote!(::sqlm_postgres::types::EnumVariant<#name>));
            }

            let enum_struct = if is_array {
                quote! { ::sqlm_postgres::types::Array<Vec<::sqlm_postgres::types::Enum<(#(#enum_variants,)*)>>> }
            } else {
                quote! { ::sqlm_postgres::types::Primitive<::sqlm_postgres::types::Enum<(#(#enum_variants,)*)>> }
            };
            return quote! {
                ::sqlm_postgres::Sql::<'_, #enum_struct, _> {
                    query: #result,
                    parameters: &[#(&(#typed_parameters),)*],
                    marker: ::std::marker::PhantomData,
                }
            }
            .into();
        } else if let Some((ty, _, is_array)) = postgres_to_rust_type(ty) {
            if is_array {
                return quote! {
                    ::sqlm_postgres::Sql::<'_, ::sqlm_postgres::types::Array<#ty>, _> {
                        query: #result,
                        parameters: &[#(&(#typed_parameters),)*],
                        marker: ::std::marker::PhantomData,
                    }
                }
                .into();
            } else {
                return quote! {
                    ::sqlm_postgres::Sql::<'_, ::sqlm_postgres::types::Primitive<#ty>, _> {
                        query: #result,
                        parameters: &[#(&(#typed_parameters),)*],
                        marker: ::std::marker::PhantomData,
                    }
                }
                .into();
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

    let mut columns = stmt.columns().iter().collect::<Vec<_>>();
    columns.sort_by_key(|c| c.name());

    let mut struct_columns: Vec<Type> = Vec::with_capacity(columns.len());
    for column in columns {
        let ty = column.type_();
        let name = const_name(column.name());
        if let Some((is_array, variants)) = enum_type(ty) {
            let mut enum_variants: Vec<Type> = Vec::with_capacity(variants.len());
            for variant in variants {
                let name = const_name(&variant);
                enum_variants.push(parse_quote!(::sqlm_postgres::types::EnumVariant<#name>));
            }

            if is_array {
                struct_columns.push(parse_quote!(::sqlm_postgres::types::StructColumn<Vec<::sqlm_postgres::types::Enum<(#(#enum_variants,)*)>>, #name>));
            } else {
                struct_columns.push(parse_quote!(::sqlm_postgres::types::StructColumn<::sqlm_postgres::types::Enum<(#(#enum_variants,)*)>, #name>));
            }
        } else if let Some((ty, _, _)) = postgres_to_rust_type(ty) {
            struct_columns.push(parse_quote!(::sqlm_postgres::types::StructColumn<#ty, #name>));
        } else {
            return syn::Error::new(
                input.query.span(),
                format!("unsupported postgres type: {ty:?}"),
            )
            .into_compile_error()
            .into();
        };
    }

    let type_struct = quote! { ::sqlm_postgres::types::Struct<(#(#struct_columns,)*)> };
    quote! {
        ::sqlm_postgres::Sql::<'_, #type_struct, _> {
            query: #result,
            parameters: &[#(&(#typed_parameters),)*],
            marker: ::std::marker::PhantomData,
        }
    }
    .into()
}

fn postgres_to_rust_type(
    ty: &postgres::types::Type,
) -> Option<(proc_macro2::TokenStream, proc_macro2::TokenStream, bool)> {
    use postgres::types::{FromSql, Kind};

    if let Kind::Array(ty) = ty.kind() {
        // reject nested array
        if matches!(ty.kind(), Kind::Array(_)) {
            return None;
        }

        return postgres_to_rust_type(ty).map(|(ty, _, _)| (quote!(Vec<#ty>), quote!([#ty]), true));
    }

    match ty {
        ty if <String as FromSql>::accepts(ty) => Some((quote!(String), quote!(str), false)),
        ty if <i64 as FromSql>::accepts(ty) => Some((quote!(i64), quote!(i64), false)),
        ty if <i32 as FromSql>::accepts(ty) => Some((quote!(i32), quote!(i32), false)),
        ty if <f64 as FromSql>::accepts(ty) => Some((quote!(f64), quote!(f64), false)),
        ty if <f32 as FromSql>::accepts(ty) => Some((quote!(f32), quote!(f32), false)),
        ty if <bool as FromSql>::accepts(ty) => Some((quote!(bool), quote!(bool), false)),
        ty if <Vec<u8> as FromSql>::accepts(ty) => Some((
            quote!(::sqlm_postgres::types::Bytea),
            quote!(::sqlm_postgres::types::Bytea),
            false,
        )),

        // serde_json::Value
        #[cfg(feature = "json")]
        ty if <::serde_json::Value as FromSql>::accepts(ty) => Some((
            quote!(::serde_json::Value),
            quote!(::serde_json::Value),
            false,
        )),

        // time::Date
        #[cfg(feature = "time")]
        ty if <::time::Date as FromSql>::accepts(ty) => {
            Some((quote!(::time::Date), quote!(::time::Date), false))
        }

        // time::OffsetDateTime
        #[cfg(feature = "time")]
        ty if <::time::OffsetDateTime as FromSql>::accepts(ty) => Some((
            quote!(::time::OffsetDateTime),
            quote!(::time::OffsetDateTime),
            false,
        )),

        // uuid::Uuid
        #[cfg(feature = "uuid")]
        ty if <::uuid::Uuid as FromSql>::accepts(ty) => {
            Some((quote!(::uuid::Uuid), quote!(::uuid::Uuid), false))
        }

        // pgvector::Vector
        #[cfg(feature = "pgvector")]
        ty if <::pgvector::Vector as FromSql>::accepts(ty) => Some((
            quote!(::pgvector::Vector),
            quote!(::pgvector::Vector),
            false,
        )),

        // Unsupported
        _ => None,
    }
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

/// Prepares `query` on the cached connection (connecting first if necessary) to retrieve its
/// parameter and column types. Errors are returned as messages ready to be reported at the query.
fn prepare(query: &str) -> Result<Statement, String> {
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

    match cached.client.prepare(query) {
        Err(err) if err.is_closed() => {
            // The connection died since it was last used, e.g. due to a server-side timeout.
            cached.client = connect(&cached.database_url)?;
            cached.client.prepare(query).map_err(prepare_error)
        }
        result => result.map_err(prepare_error),
    }
}

fn prepare_error(err: postgres::Error) -> String {
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
