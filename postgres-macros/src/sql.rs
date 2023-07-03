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

#[derive(Debug, Default)]
pub struct Opts {
    pub skip_query_check: bool,
}

pub fn sql(item: TokenStream, opts: Opts) -> TokenStream {
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

    let Ok(tokens) = parser::parse(&query)
    else {
        return syn::Error::new(input.query.span(), "invalid format string")
            .into_compile_error()
            .into();
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
                let Some(param) = unnamed_arguments.get_mut(next_arg)
                else {
                    return syn::Error::new(input.query.span(), format!("missing argument for position {next_arg}"))
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
                let Some(param) = unnamed_arguments.get_mut(ix)
                else {
                    return syn::Error::new(input.query.span(), format!("missing argument for index {ix}"))
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

    #[cfg(not(feature = "comptime"))]
    let _ = opts; // prevent unused warning
    #[cfg(feature = "comptime")]
    if !opts.skip_query_check {
        use std::str::FromStr;

        use postgres::Config;

        let Ok(database_url) = dotenvy::var("DATABASE_URL")
        else {
            return syn::Error::new(
                input.query.span(),
                "compile-time query checks require DATABASE_URL environment variable to be defined"
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

        let mut columns = Vec::with_capacity(stmt.columns().len());
        for column in stmt.columns() {
            let ty = column.type_();
            let Some(ty) = postgres_to_rust_type(ty)
            else {
                return syn::Error::new(
                    input.query.span(),
                    format!("unsupported postgres type: {ty:?}"),
                )
                .into_compile_error()
                .into();
            };

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

            columns.push(quote! {
                impl ::sqlm_postgres::HasColumn<#ty, #name> for Cols {}
            });
        }

        let mut typed_parameters = Vec::with_capacity(parameters.len());
        for (ty, param) in stmt.params().iter().zip(parameters) {
            let Some(ty) = postgres_to_rust_type(ty)
            else {
                return syn::Error::new(
                    input.query.span(),
                    format!("unsupporte postgres type: {ty:?}"),
                )
                .into_compile_error()
                .into();
            };

            typed_parameters.push(quote! {
                #ty::from(#param)
            });
        }

        return quote! {
            {
                pub struct Cols;

                #(#columns)*

                ::sqlm_postgres::Sql::<'_, Cols, _> {
                    query: #result,
                    parameters: &[#(&#typed_parameters,)*],
                    marker: ::std::marker::PhantomData,
                }
            }
        }
        .into();
    }

    quote! {
        ::sqlm_postgres::Sql::<'_, ::sqlm_postgres::AnyCols, _> {
            query: #result,
            parameters: &[#(&{#parameters},)*],
            marker: ::std::marker::PhantomData,
        }
    }
    .into()
}

#[cfg(feature = "comptime")]
fn postgres_to_rust_type(ty: &postgres::types::Type) -> Option<syn::TypePath> {
    use postgres::types::{FromSql, Kind};
    use syn::parse_quote;

    match ty {
        // String
        ty if <String as FromSql>::accepts(ty) => Some(parse_quote!(String)),

        // i64
        ty if <i64 as FromSql>::accepts(ty) => Some(parse_quote!(i64)),

        // i32
        ty if <i32 as FromSql>::accepts(ty) => Some(parse_quote!(i32)),

        // bool
        ty if <bool as FromSql>::accepts(ty) => Some(parse_quote!(bool)),

        // serde_json::Value
        #[cfg(feature = "json")]
        ty if <serde_json::Value as FromSql>::accepts(ty) => {
            Some(parse_quote!(::serde_json::Value))
        }

        // time::OffsetDateTime
        #[cfg(feature = "time")]
        ty if <time::OffsetDateTime as FromSql>::accepts(ty) => {
            Some(parse_quote!(::time::OffsetDateTime))
        }

        // uuid::Uuid
        #[cfg(feature = "uuid")]
        ty if <uuid::Uuid as FromSql>::accepts(ty) => Some(parse_quote!(::uuid::Uuid)),

        // Enum
        ty if matches!(ty.kind(), Kind::Enum(_)) => {
            let oid = ty.oid() as usize;
            Some(parse_quote!(::sqlm_postgres::Enum<#oid>))
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
