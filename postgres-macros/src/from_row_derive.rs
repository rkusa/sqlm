use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    parse_quote, token, Attribute, Data, DataStruct, DeriveInput, Error, Expr, Fields, Path,
    PathArguments, Type,
};

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

    let mut new_generics = generics.clone();
    new_generics.params.push(parse_quote!(Cols));
    let (impl_generics_with_cols, _, _) = new_generics.split_for_impl();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut where_predicates = where_clause
        .map(|w| w.predicates.clone())
        .unwrap_or_default();

    let mut field_assignments = Vec::with_capacity(fields.named.len());

    for f in fields.named.iter() {
        let opts = extract_field_options(&f.attrs)?;
        let ident = f.ident.as_ref().unwrap();
        let (ty, kind) = extract_inner_type(&f.ty)?;

        let name = ident.to_string();
        #[cfg(not(nightly_column_names))]
        let name = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::default();
            name.hash(&mut hasher);
            hasher.finish() as usize
        };
        where_predicates.push(parse_quote!(Cols: ::sqlm_postgres::HasColumn<#ty, #name>));

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
        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics_with_cols ::sqlm_postgres::FromRow<Cols> for #ident #ty_generics
            where
                #where_predicates
            {
                fn from_row(row: ::sqlm_postgres::Row<Cols>) -> Result<Self, ::sqlm_postgres::tokio_postgres::Error> {
                    Ok(Self {
                        #(#field_assignments)*
                    })
                }
            }

            #[automatically_derived]
            impl #impl_generics_with_cols ::sqlm_postgres::Query<Cols> for #ident #ty_generics
            where
                Cols: Send + Sync,
                #where_predicates
            {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, Cols, Self>,
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
                        Ok(::sqlm_postgres::FromRow::<Cols>::from_row(row.into())?)
                    })
                }
            }
        })
    }
    #[cfg(not(feature = "comptime"))]
    {
        let _ = impl_generics_with_cols;
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
            impl #impl_generics ::sqlm_postgres::Query<#ident> for #ident #ty_generics #where_clause {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, #ident, Self>,
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

            #[automatically_derived]
            impl #impl_generics ::sqlm_postgres::Query<#ident> for Option<#ident #ty_generics> #where_clause {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, #ident, Self>,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<Self, ::sqlm_postgres::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        let row = if let Some(tx) = sql.transaction {
                            let stmt = tx.prepare_cached(sql.query).await?;
                            tx.query_opt(&stmt, sql.parameters).await?
                        } else {
                            let conn = ::sqlm_postgres::connect().await?;
                            let stmt = conn.prepare_cached(sql.query).await?;
                            conn.query_opt(&stmt, sql.parameters).await?
                        };
                        match row {
                            Some(row) => Ok(Some(::sqlm_postgres::FromRow::<#ident>::from_row(row.into())?)),
                            None => Ok(None),
                        }
                    })
                }
            }

            #[automatically_derived]
            impl #impl_generics ::sqlm_postgres::Query<#ident> for Vec<#ident #ty_generics> #where_clause {
                fn query<'a>(
                    sql: &'a ::sqlm_postgres::Sql<'a, #ident, Self>,
                ) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<Self, ::sqlm_postgres::Error>> + Send + 'a>> {
                    Box::pin(async move {
                        let rows = if let Some(tx) = sql.transaction {
                            let stmt = tx.prepare_cached(sql.query).await?;
                            tx.query(&stmt, sql.parameters).await?
                        } else {
                            let conn = ::sqlm_postgres::connect().await?;
                            let stmt = conn.prepare_cached(sql.query).await?;
                            conn.query(&stmt, sql.parameters).await?
                        };
                        rows.into_iter()
                            .map(|row| ::sqlm_postgres::FromRow::<#ident>::from_row(row.into()).map_err(::sqlm_postgres::Error::from))
                            .collect()
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
