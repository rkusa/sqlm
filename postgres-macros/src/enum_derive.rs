use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Data, DataEnum, DeriveInput, Error, LitStr, Path, Type, parse_quote, token};

use crate::const_name;
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
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut variants = variants
        .into_iter()
        .map(|v| {
            let vopts = extract_variant_options(&v.attrs)?;
            let name = v.ident.to_string();
            let name = if let Some(rename) = vopts.rename {
                rename
            } else if let Some(rename_all) = &opts.rename_all {
                rename_all.apply(&name)
            } else {
                name
            };
            Ok(name)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    variants.sort();

    let mut enum_variants: Vec<Type> = Vec::with_capacity(variants.len());
    for name in variants {
        let name = const_name(&name);
        enum_variants.push(parse_quote!(::sqlm_postgres::types::EnumVariant<#name>));
    }

    let enum_struct = quote! { ::sqlm_postgres::types::Enum<(#(#enum_variants,)*)> };
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::sqlm_postgres::SqlType for #ident #ty_generics #where_clause {
            type Type = #enum_struct;
        }
    })
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
