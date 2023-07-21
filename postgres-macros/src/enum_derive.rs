use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_quote, token, Attribute, Data, DataEnum, DeriveInput, Error, LitStr, Path};

use crate::rename::RenameAll;

pub fn expand_derive_enum(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        vis: _,
        ident,
        generics,
        data,
    } = input;

    let Data::Enum(DataEnum { variants, .. }) = data else {
        return Err(Error::new(
            ident.span(),
            "Enum can only be derived from enums",
        ));
    };

    let opts = extract_options(&attrs)?;

    let mut new_generics = generics.clone();
    new_generics.params.push(parse_quote!(Cols));
    let (impl_generics_with_cols, _, _) = new_generics.split_for_impl();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut where_predicates = where_clause
        .map(|w| w.predicates.clone())
        .unwrap_or_default();

    let mut has_variant_impls = Vec::with_capacity(variants.len());
    let n = variants.len();

    for v in variants {
        let vopts = extract_variant_options(&v.attrs)?;
        let mut name = v.ident.to_string();

        if let Some(rename) = vopts.rename {
            name = rename;
        } else if let Some(rename_all) = &opts.rename_all {
            name = rename_all.apply(&name);
        }

        #[cfg(not(nightly_column_names))]
        let name = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::default();
            name.hash(&mut hasher);
            hasher.finish() as usize
        };

        has_variant_impls.push(quote! {
            impl #impl_generics ::sqlm_postgres::HasVariant<#n, #name> for #ident #ty_generics #where_clause {}
        });
        where_predicates.push(parse_quote!(Cols: ::sqlm_postgres::HasVariant<#n, #name>));
    }

    #[cfg(feature = "comptime")]
    {
        Ok(quote! {
            #(
                #has_variant_impls
            )*

            #[automatically_derived]
            impl #impl_generics_with_cols ::sqlm_postgres::FromRow<::sqlm_postgres::Literal<Cols>> for #ident #ty_generics
            where
                #where_predicates
            {
                fn from_row(row: ::sqlm_postgres::Row<::sqlm_postgres::Literal<Cols>>) -> Result<Self, ::sqlm_postgres::tokio_postgres::Error> {
                    row.try_get(0)
                }
            }

            #[automatically_derived]
            impl #impl_generics_with_cols ::sqlm_postgres::Query<::sqlm_postgres::Literal<Cols>> for #ident #ty_generics
            where
                Cols: Send + Sync,
                #where_predicates
            {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, ::sqlm_postgres::Literal<Cols>, Self>,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<Self, ::sqlm_postgres::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        let row = if let Some(tx) = sql.transaction {
                            let stmt = tx.prepare_cached(sql.query).await?;
                            tx.query_one(&stmt, sql.parameters).await?
                        } else {
                            let conn = ::sqlm_postgres::connect().await?;
                            let stmt = conn.prepare_cached(sql.query).await?;
                            conn.query_one(&stmt, sql.parameters).await?
                        };
                        Ok(::sqlm_postgres::FromRow::<::sqlm_postgres::Literal<Cols>>::from_row(row.into())?)
                    })
                }
            }
        })
    }
    #[cfg(not(feature = "comptime"))]
    {
        let _ = impl_generics_with_cols;
        Ok(quote! {
            #(
                #has_variant_impls
            )*

            #[automatically_derived]
            impl #impl_generics ::sqlm_postgres::Query<::sqlm_postgres::Literal<#ident>> for #ident #ty_generics #where_clause {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, ::sqlm_postgres::Literal<#ident>, Self>,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<Self, ::sqlm_postgres::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        let row = if let Some(tx) = sql.transaction {
                            let stmt = tx.prepare_cached(sql.query).await?;
                            tx.query_one(&stmt, sql.parameters).await?
                        } else {
                            let conn = ::sqlm_postgres::connect().await?;
                            let stmt = conn.prepare_cached(sql.query).await?;
                            conn.query_one(&stmt, sql.parameters).await?
                        };
                        Ok(row.try_get(0)?)
                    })
                }
            }
        })
    }
}

#[derive(Default)]
struct Options {
    rename_all: Option<RenameAll>,
}

fn extract_options(attrs: &[Attribute]) -> Result<Options, Error> {
    let mut opts = Options::default();

    for attr in attrs {
        if !attr.path().is_ident("postgres") {
            continue;
        }

        for opt in attr.parse_args_with(Punctuated::<OptionExpr, token::Comma>::parse_terminated)? {
            if opt.key.is_ident("rename_all") {
                let Some(value) = opt.value else {
                    return Err(Error::new_spanned(
                        opt.value,
                        "rename_all must have a value",
                    ));
                };

                let Ok(rename_all) = RenameAll::from_str(&value.value()) else {
                    return Err(Error::new_spanned(value, "invalid rename_all rule"));
                };

                opts.rename_all = Some(rename_all);
            }

            // ignore unknown options as they might be part of the FromSql/ToSql derive
        }
    }

    Ok(opts)
}

#[derive(Default)]
struct VariantOptions {
    rename: Option<String>,
}

fn extract_variant_options(attrs: &[Attribute]) -> Result<VariantOptions, Error> {
    let mut opts = VariantOptions::default();

    for attr in attrs {
        if !attr.path().is_ident("postgres") {
            continue;
        }

        for opt in attr.parse_args_with(Punctuated::<OptionExpr, token::Comma>::parse_terminated)? {
            if opt.key.is_ident("name") {
                let Some(value) = opt.value else {
                    return Err(Error::new_spanned(opt.value, "rename must have a value"));
                };

                opts.rename = Some(value.value());
            }

            // ignore unknown options as they might be part of the FromSql/ToSql derive
        }
    }

    Ok(opts)
}

#[derive(Debug, Hash)]
struct OptionExpr {
    key: Path,
    value: Option<LitStr>,
}

impl Parse for OptionExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        let value = if Option::<token::Eq>::parse(input)?.is_some() {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(OptionExpr { key, value })
    }
}
