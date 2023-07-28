use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Write;

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse_macro_input, Expr, LitStr};

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

    #[cfg(feature = "comptime")]
    {
        use std::str::FromStr;

        use postgres::types::Kind;
        use postgres::Config;
        use syn::{parse_quote, Type};

        let Ok(database_url) = dotenvy::var("DATABASE_URL") else {
            return syn::Error::new(
                input.query.span(),
                "compile-time query checks require DATABASE_URL environment variable to be defined",
            )
            .into_compile_error()
            .into();
        };
        let config = match Config::from_str(&database_url) {
            Ok(config) => config,
            Err(err) => {
                return syn::Error::new(
                    input.query.span(),
                    format!("failed to parse connection config from DATABASE_URL: {err}"),
                )
                .into_compile_error()
                .into();
            }
        };

        // TODO: allow TLS?
        let mut client = match config.connect(postgres::NoTls) {
            Ok(client) => client,
            Err(err) => {
                return syn::Error::new(
                    input.query.span(),
                    format!("failed to connect to postgres (using DATABASE_URL): {err}"),
                )
                .into_compile_error()
                .into();
            }
        };

        let stmt = match client.prepare(&result) {
            Ok(stmt) => stmt,
            Err(err) => {
                return syn::Error::new(input.query.span(), format!("query failed: {err}"))
                    .into_compile_error()
                    .into();
            }
        };

        let mut typed_parameters = Vec::with_capacity(parameters.len());
        for (ty, param) in stmt.params().iter().zip(parameters) {
            let enum_data = match ty.kind() {
                Kind::Enum(variants) => Some((false, variants)),
                Kind::Array(ty) => {
                    if let Kind::Enum(variants) = ty.kind() {
                        Some((true, variants))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some((is_array, variants)) = enum_data {
                let mut enum_variants: Vec<Type> = Vec::with_capacity(variants.len());
                for variant in variants {
                    #[cfg(not(nightly_column_names))]
                    let name = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::default();
                        variant.hash(&mut hasher);
                        hasher.finish() as usize
                    };
                    #[cfg(nightly_column_names)]
                    let name = variant;
                    enum_variants.push(parse_quote!(::sqlm_postgres::types::EnumVariant<#name>));
                }

                let enum_struct = quote! { ::sqlm_postgres::types::Enum<(#(#enum_variants,)*)> };
                let type_check = if is_array {
                    quote! {
                        {
                            const fn assert_type<T: ::sqlm_postgres::SqlType<Type = #enum_struct>>(_: &[T]) {}
                            assert_type(&(#param));
                        }
                    }
                } else {
                    quote! {
                        {
                            const fn assert_type<T: ::sqlm_postgres::SqlType<Type = #enum_struct>>(_: &T) {}
                            assert_type(&(#param));
                        }
                    }
                };

                typed_parameters.push(quote! {
                    {
                        #type_check
                        &(#param)
                    }
                });
                continue;
            }

            let Some((ty_owned, ty_borrowed)) = postgres_to_rust_type(ty) else {
                return syn::Error::new(
                    input.query.span(),
                    format!("unsupporte postgres type: {ty:?}"),
                )
                .into_compile_error()
                .into();
            };

            // `Option::from` is used to allow parameters to be an Option
            typed_parameters.push(quote! {
                {
                    {
                        const fn assert_type<T, S>(_: &T)
                        where
                            T: ::sqlm_postgres::SqlType<Type = S>,
                            for<'a> ::sqlm_postgres::internal::Valid<'a, #ty_borrowed, #ty_owned>: From<S>
                        {}
                        assert_type(&(#param));
                    }
                    &(#param)
                }
            });
        }

        let col_count = stmt.columns().len();
        if col_count == 1 {
            // Consider the result to be a literal
            let ty = stmt.columns()[0].type_();
            let enum_data: Option<(bool, &[String])> = match ty.kind() {
                Kind::Enum(variants) => Some((false, variants)),
                Kind::Array(ty) => {
                    if let Kind::Enum(variants) = ty.kind() {
                        Some((true, variants))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some((is_array, variants)) = enum_data {
                let mut enum_variants: Vec<Type> = Vec::with_capacity(variants.len());
                for variant in variants {
                    #[cfg(not(nightly_column_names))]
                    let name = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::default();
                        variant.hash(&mut hasher);
                        hasher.finish() as usize
                    };
                    #[cfg(nightly_column_names)]
                    let name = variant;
                    enum_variants.push(parse_quote!(::sqlm_postgres::types::EnumVariant<#name>));
                }

                let enum_struct = if is_array {
                    quote! { Vec<::sqlm_postgres::types::Enum<(#(#enum_variants,)*)>> }
                } else {
                    quote! { ::sqlm_postgres::types::Enum<(#(#enum_variants,)*)> }
                };
                return quote! {
                    {
                        ::sqlm_postgres::Sql::<'_, ::sqlm_postgres::types::Literal<#enum_struct>, _> {
                            query: #result,
                            parameters: &[#(&(#typed_parameters),)*],
                            transaction: None,
                            marker: ::std::marker::PhantomData,
                        }
                    }
                }
                .into();
            } else if let Some((ty, _)) = postgres_to_rust_type(ty) {
                return quote! {
                    {
                        ::sqlm_postgres::Sql::<'_, ::sqlm_postgres::types::Literal<#ty>, _> {
                            query: #result,
                            parameters: &[#(&(#typed_parameters),)*],
                            transaction: None,
                            marker: ::std::marker::PhantomData,
                        }
                    }
                }
                .into();
            } else {
                return syn::Error::new(
                    input.query.span(),
                    format!("unsupported postgres type: {ty:?}"),
                )
                .into_compile_error()
                .into();
            }
        }

        let mut columns = Vec::with_capacity(stmt.columns().len());
        for column in stmt.columns() {
            let ty = column.type_();

            #[cfg(not(nightly_column_names))]
            let name = {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::default();
                column.name().hash(&mut hasher);
                hasher.finish() as usize
            };
            #[cfg(nightly_column_names)]
            let name = column.name();

            // TODO: array
            let enum_data = match ty.kind() {
                Kind::Enum(variants) => Some((false, variants)),
                Kind::Array(ty) => {
                    if let Kind::Enum(variants) = ty.kind() {
                        Some((true, variants))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some((is_array, variants)) = enum_data {
                let mut enum_variants: Vec<Type> = Vec::with_capacity(variants.len());
                for variant in variants {
                    #[cfg(not(nightly_column_names))]
                    let name = {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::default();
                        variant.hash(&mut hasher);
                        hasher.finish() as usize
                    };
                    #[cfg(nightly_column_names)]
                    let name = variant;
                    enum_variants.push(parse_quote!(::sqlm_postgres::types::EnumVariant<#name>));
                }

                if is_array {
                    columns.push(quote! {
                        impl ::sqlm_postgres::types::HasColumn<Vec<::sqlm_postgres::types::Enum<(#(#enum_variants,)*)>>, #name> for Cols {}
                    });
                } else {
                    columns.push(quote! {
                    impl ::sqlm_postgres::types::HasColumn<::sqlm_postgres::types::Enum<(#(#enum_variants,)*)>, #name> for Cols {}
                });
                }
            } else if let Some((ty, _)) = postgres_to_rust_type(ty) {
                columns.push(quote! {
                    impl ::sqlm_postgres::types::HasColumn<#ty, #name> for Cols {}
                });
            } else {
                return syn::Error::new(
                    input.query.span(),
                    format!("unsupported postgres type: {ty:?}"),
                )
                .into_compile_error()
                .into();
            };
        }

        quote! {
            {
                pub struct Cols;

                #(#columns)*

                ::sqlm_postgres::Sql::<'_, ::sqlm_postgres::types::Struct<Cols>, _> {
                    query: #result,
                    parameters: &[#(&(#typed_parameters),)*],
                    transaction: None,
                    marker: ::std::marker::PhantomData,
                }
            }
        }
        .into()
    }

    #[cfg(not(feature = "comptime"))]
    {
        quote! {
            ::sqlm_postgres::Sql::<'_, _, _> {
                query: #result,
                parameters: &[#(&(#parameters),)*],
                transaction: None,
                marker: ::std::marker::PhantomData,
            }
        }
        .into()
    }
}

#[cfg(feature = "comptime")]
fn postgres_to_rust_type(
    ty: &postgres::types::Type,
) -> Option<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    use postgres::types::{FromSql, Kind};

    if let Kind::Array(ty) = ty.kind() {
        // reject nested array
        if matches!(ty.kind(), Kind::Array(_)) {
            return None;
        }

        return postgres_to_rust_type(ty).map(|(ty, _)| (quote!(Vec<#ty>), quote!([#ty])));
    }

    match ty {
        ty if <String as FromSql>::accepts(ty) => Some((quote!(String), quote!(str))),
        ty if <i64 as FromSql>::accepts(ty) => Some((quote!(i64), quote!(i64))),
        ty if <i32 as FromSql>::accepts(ty) => Some((quote!(i32), quote!(i32))),
        ty if <bool as FromSql>::accepts(ty) => Some((quote!(bool), quote!(bool))),
        ty if <Vec<u8> as FromSql>::accepts(ty) => Some((quote!(Vec<u8>), quote!([u8]))),

        // serde_json::Value
        #[cfg(feature = "json")]
        ty if <::serde_json::Value as FromSql>::accepts(ty) => {
            Some((quote!(::serde_json::Value), quote!(::serde_json::Value)))
        }

        // time::OffsetDateTime
        #[cfg(feature = "time")]
        ty if <::time::OffsetDateTime as FromSql>::accepts(ty) => Some((
            quote!(::time::OffsetDateTime),
            quote!(::time::OffsetDateTime),
        )),

        // uuid::Uuid
        #[cfg(feature = "uuid")]
        ty if <::uuid::Uuid as FromSql>::accepts(ty) => {
            Some((quote!(::uuid::Uuid), quote!(::uuid::Uuid)))
        }

        // Unsupported
        _ => None,
    }
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
