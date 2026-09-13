use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

use entity_lookup::into_looked_up_impl;

mod entity_lookup;

#[proc_macro_derive(IntoLookedUp, attributes(elf))]
pub fn into_looked_up(item: TokenStream) -> TokenStream {
    match into_looked_up_impl(&parse_macro_input!(item as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
