use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DataStruct, DeriveInput, Error, Fields, PathArguments, Type};

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
        return Err(Error::new(ident.span(), "FromRow can only be derived from named structs"));
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
            Kind::Other => field_assignments.push(quote! {
                #(#attrs)*
                #ident: {
                    let v: Option<#ty> = row.try_get(#name)?;
                    v.unwrap_or_default()
                },
            }),
        }
    }

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
        impl #impl_generics ::sqlm_postgres::FromRow<::sqlm_postgres::AnyCols> for #ident #ty_generics #where_clause {
            fn from_row(row: ::sqlm_postgres::Row<::sqlm_postgres::AnyCols>) -> Result<Self, ::sqlm_postgres::tokio_postgres::Error> {
                Ok(Self {
                    #(#field_assignments)*
                })
            }
        }
    })
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
