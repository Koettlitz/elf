//! This trait provides macros, that go along with the bevy_elf crate.
//! It is recommended to use `bevy_elf`, which re-exports this crates macros instead of using this
//! crate directly.

use std::ops;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    DeriveInput, Ident, Item, ItemEnum, ItemStruct, TypePath, parse_macro_input, spanned::Spanned,
};

use crate::{from_def::from_def_impl, spec::SpecArgs};

mod from_def;
mod spec;

#[cfg(test)]
mod test;

const ELF_MODULE_PATH: &str = "bevy_elf";

#[proc_macro_attribute]
pub fn asset_spec(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = match syn::parse(item.clone()) {
        Ok(item) => item,
        Err(e) => {
            let error = e.to_compile_error();
            let item: proc_macro2::TokenStream = item.into();
            return quote! {
                #item
                #error
            }
            .into();
        }
    };
    let type_ident = match &item {
        Item::Struct(ItemStruct { ident, .. }) => ident,
        Item::Enum(ItemEnum { ident, .. }) => ident,
        _ => {
            let error = syn::Error::new_spanned(
                &item,
                "`asset_spec` attribute is only valid for structs and enums",
            )
            .to_compile_error();
            return quote! {
                #item
                #error
            }
            .into();
        }
    };
    let asset_module = match CratePath::try_from(ELF_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.to_compile_error().into(),
    };
    let args = parse_macro_input!(attr as SpecArgs);
    let base_path = &args.base_path;
    let extension = args
        .extension
        .as_ref()
        .map(|e| quote!(Some(#e)))
        .unwrap_or_else(|| quote!(None));

    quote! {
        #item

        impl #asset_module::AssetPathSpec for #type_ident {
            const BASE_PATH: &'static str = #base_path;
            const EXTENSION: Option<&'static str> = #extension;
        }

        impl #asset_module::HasResolver for #type_ident {
            type Resolver = #asset_module::ResolverSpec<Self>;

            fn resolver() -> Self::Resolver {
                #asset_module::ResolverSpec::<Self>::default()
            }
        }
    }
    .into()
}

#[proc_macro_derive(FromDef, attributes(elf))]
pub fn from_def(item: TokenStream) -> TokenStream {
    match from_def_impl(parse_macro_input!(item as DeriveInput)) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// A [`syn::Path`] whose first segment is rewritten to the correct crate name.
///
/// Resolution happens in [`CratePath::try_from`] using
/// [`proc_macro_crate::crate_name`].
#[derive(Clone)]
struct CratePath(syn::Path);

impl TryFrom<&str> for CratePath {
    type Error = syn::Error;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let path: syn::Path = syn::parse_str(path)?;
        Self::try_from(path)
    }
}

impl TryFrom<syn::Path> for CratePath {
    type Error = syn::Error;

    fn try_from(mut path: syn::Path) -> Result<Self, Self::Error> {
        let first_segment = match path.segments.first_mut() {
            Some(segment) => segment,
            None => {
                return Err(syn::Error::new(
                    path.span(),
                    "wtf is this? Comon man! Don't gimme that empty syn::Path abomination! I can't...",
                ));
            }
        };
        let crate_string = first_segment.ident.to_string();
        let span = first_segment.ident.span();
        first_segment.ident = match crate_name(&crate_string) {
            Ok(FoundCrate::Itself) => Ident::new("crate", span),
            Ok(FoundCrate::Name(name)) => Ident::new(&name, span),
            Err(e) => {
                return Err(syn::Error::new(
                    span,
                    format!("could not resolve crate `{crate_string}`: {e}"),
                ));
            }
        };
        Ok(Self(path))
    }
}

impl ops::Deref for CratePath {
    type Target = syn::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<syn::Path> for CratePath {
    fn as_ref(&self) -> &syn::Path {
        &self.0
    }
}

impl From<CratePath> for syn::Path {
    fn from(p: CratePath) -> Self {
        p.0
    }
}

impl ToTokens for CratePath {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

fn bevy_asset_mod_path() -> syn::Result<proc_macro2::TokenStream> {
    if let Ok(found) = crate_name("bevy_asset") {
        return Ok(match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name, Span::call_site());
                quote!(#ident)
            }
        });
    }
    match crate_name("bevy") {
        Ok(FoundCrate::Itself) => Ok(quote!(crate::asset)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(#ident::asset))
        }
        Err(e) => Err(syn::Error::new(
            Span::call_site(),
            format!("could not resolve `bevy_asset` or `bevy` (needed for `LoadContext`): {e}"),
        )),
    }
}

fn is_self(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(TypePath {
            qself: None, path, ..
        }) => path.is_ident("Self"),
        _ => false,
    }
}
