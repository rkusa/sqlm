#![cfg_attr(nightly_column_names, feature(adt_const_params))]
#![cfg_attr(nightly_column_names, allow(incomplete_features))]

mod from_row;
mod parser;
mod sql;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(FromRow, attributes(sqlm))]
pub fn derive_fromsql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    from_row::expand_derive_from_row(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro]
pub fn sql(item: TokenStream) -> TokenStream {
    sql::sql(item, sql::Opts::default())
}

#[proc_macro]
pub fn sql_unchecked(item: TokenStream) -> TokenStream {
    sql::sql(
        item,
        sql::Opts {
            skip_query_check: true,
        },
    )
}
