#![cfg_attr(nightly_column_names, feature(adt_const_params))]
#![cfg_attr(nightly_column_names, allow(incomplete_features))]

#[cfg(feature = "comptime")]
mod enum_derive;
mod from_row_derive;
mod parser;
#[cfg(feature = "comptime")]
mod rename;
mod sql;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(FromRow, attributes(sqlm))]
pub fn derive_fromsql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    from_row_derive::expand_derive_from_row(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[cfg(feature = "comptime")]
#[proc_macro_derive(Enum, attributes(postgres))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    enum_derive::expand_derive_enum(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[cfg(not(feature = "comptime"))]
#[proc_macro_derive(Enum, attributes(postgres))]
pub fn derive_enum(_input: TokenStream) -> TokenStream {
    quote::quote! {}.into()
}

#[proc_macro]
pub fn sql(item: TokenStream) -> TokenStream {
    sql::sql(item)
}
