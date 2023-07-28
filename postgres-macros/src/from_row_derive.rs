use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    parse_quote, token, Attribute, Data, DataStruct, DeriveInput, Error, Expr, Fields, Path,
    PathArguments, Type,
};

use crate::const_name;

pub fn expand_derive_from_row(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs: _,
        vis: _,
        ident,
        generics,
        data,
    } = input;

    let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = data
    else {
        return Err(Error::new(
            ident.span(),
            "FromRow can only be derived from named structs",
        ));
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut struct_columns: Vec<Type> = Vec::with_capacity(fields.named.len());
    let mut field_assignments = Vec::with_capacity(fields.named.len());

    let mut fields = fields
        .named
        .into_iter()
        .map(|f| (f.ident.as_ref().unwrap().to_string(), f))
        .collect::<Vec<_>>();
    fields.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    for (name, f) in fields {
        let opts = extract_field_options(&f.attrs)?;
        let ident = f.ident.as_ref().unwrap();
        let (ty, kind) = extract_inner_type(&f.ty)?;

        let name = const_name(&name);
        struct_columns.push(parse_quote!(::sqlm_postgres::types::StructColumn<<#ty as ::sqlm_postgres::SqlType>::Type, #name>));

        // Forward only certain args
        let attrs = f
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("cfg"))
            .collect::<Vec<_>>();

        let name = ident.to_string();
        match kind {
            Kind::Option => field_assignments.push(quote! {
                #(#attrs)*
                #ident: row.try_get(#name)?,
            }),
            Kind::Other => {
                let default = if let Some(default) = opts.default {
                    quote! { v.unwrap_or_else(|| { #default }.into()) }
                } else {
                    quote! { v.unwrap_or_default() }
                };
                field_assignments.push(quote! {
                    #(#attrs)*
                    #ident: {
                        let v: Option<#ty> = row.try_get(#name)?;
                        #default
                    },
                })
            }
        }
    }

    #[cfg(feature = "comptime")]
    {
        let _ = impl_generics;
        let type_struct = quote! { ::sqlm_postgres::types::Struct<(#(#struct_columns,)*)> };
        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics ::sqlm_postgres::FromRow<#type_struct> for #ident #ty_generics #where_clause {
                fn from_row(row: ::sqlm_postgres::Row<#type_struct>) -> Result<Self, ::sqlm_postgres::tokio_postgres::Error> {
                    Ok(Self {
                        #(#field_assignments)*
                    })
                }
            }

            #[automatically_derived]
            impl #impl_generics ::sqlm_postgres::Query<#type_struct> for #ident #ty_generics #where_clause {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, #type_struct, Self>,
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
                        Ok(::sqlm_postgres::FromRow::<#type_struct>::from_row(row.into())?)
                    })
                }
            }
        })
    }
    #[cfg(not(feature = "comptime"))]
    {
        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics ::sqlm_postgres::FromRow<#ident> for #ident #ty_generics #where_clause {
                fn from_row(row: ::sqlm_postgres::Row<#ident>) -> Result<Self, ::sqlm_postgres::tokio_postgres::Error> {
                    Ok(Self {
                        #(#field_assignments)*
                    })
                }
            }

            #[automatically_derived]
            impl #impl_generics ::sqlm_postgres::Query<::sqlm_postgres::types::Struct<#ident>> for #ident #ty_generics #where_clause {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, ::sqlm_postgres::types::Struct<#ident>, Self>,
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
                        Ok(::sqlm_postgres::FromRow::<#ident>::from_row(row.into())?)
                    })
                }
            }
        })
    }
}

pub(crate) enum Kind {
    Option,
    Other,
}

pub(crate) fn extract_inner_type(ty: &Type) -> syn::Result<(&Type, Kind)> {
    if let Type::Path(p) = ty {
        if p.path.segments.len() != 1 {
            return Ok((ty, Kind::Other));
        }

        let segment = &p.path.segments[0];
        if segment.ident != "Option" {
            return Ok((ty, Kind::Other));
        }

        if let PathArguments::AngleBracketed(args) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(t)) = args.args.first() {
                return Ok((t, Kind::Option));
            }
        }
    }

    Ok((ty, Kind::Other))
}

#[derive(Default)]
struct FieldOptions {
    default: Option<Expr>,
}

fn extract_field_options(attrs: &[Attribute]) -> Result<FieldOptions, Error> {
    let mut opts = FieldOptions::default();

    for attr in attrs {
        if !attr.path().is_ident("sqlm") {
            continue;
        }

        for opt in attr.parse_args_with(Punctuated::<OptionExpr, token::Comma>::parse_terminated)? {
            if opt.key.is_ident("default") {
                opts.default = Some(opt.value);
            } else {
                return Err(Error::new_spanned(opt.key, "unknown option"));
            }
        }
    }

    Ok(opts)
}

#[derive(Debug, Hash)]
struct OptionExpr {
    key: Path,
    value: Expr,
}

impl Parse for OptionExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;

        token::Eq::parse(input)?;
        let value = input.parse()?;
        Ok(OptionExpr { key, value })
    }
}
