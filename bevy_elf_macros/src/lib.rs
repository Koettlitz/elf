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

use crate::{
    from_def::{
        DefTransformResult, TypeElfAttr, derive_def_type_name, from_def_trait, generate_def_for,
        generate_def_transform,
    },
    spec::SpecArgs,
};

mod from_def;
mod spec;

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
    let input = parse_macro_input!(item as DeriveInput);
    let elf_crate_path = match CratePath::try_from(ELF_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.into_compile_error().into(),
    };
    let bevy_asset_crate = match resolve_crate_name("bevy_asset") {
        Ok(bevy_asset_crate) => bevy_asset_crate,
        Err(e) => return e.into_compile_error().into(),
    };
    let load_context_var_ident = Ident::new("ctx", Span::call_site());
    let input_ident = &input.ident;
    let from_def_trait = match from_def_trait() {
        Ok(from_def_trait) => from_def_trait,
        Err(e) => return e.into_compile_error().into(),
    };
    let def_var_ident = Ident::new("def", Span::call_site());

    let type_elf_attr = match TypeElfAttr::from_attrs(&input.attrs) {
        Ok(attr) => attr,
        Err(e) => return e.to_compile_error().into(),
    };
    let (generated_def, def_type, def_transform) = match type_elf_attr {
        Some(TypeElfAttr::DefType(def_type)) if is_self(&def_type) => (
            None,
            *def_type,
            DefTransformResult {
                transformation: def_var_ident.to_token_stream(),
                resolver_fns: Vec::new(),
            },
        ),
        Some(TypeElfAttr::DefType(def_type)) => {
            let def_transform = match generate_def_transform(
                &input,
                &def_type,
                &def_var_ident,
                &load_context_var_ident,
            ) {
                Ok(cimpl) => cimpl,
                Err(e) => return e.to_compile_error().into(),
            };
            (None, *def_type, def_transform)
        }
        other => {
            let def_attrs = if let Some(TypeElfAttr::DefAttrs(def_attrs)) = other {
                def_attrs
            } else {
                Vec::new()
            };
            let def_type_name = derive_def_type_name(&input_ident.to_string());
            let def_type = match syn::parse_str(&def_type_name) {
                Ok(def_type) => def_type,
                Err(e) => return e.to_compile_error().into(),
            };
            let generated_def = match generate_def_for(&input, &def_type, &def_attrs) {
                Ok(def) => def,
                Err(e) => return e.to_compile_error().into(),
            };
            let def_transform = match generate_def_transform(
                &input,
                &def_type,
                &def_var_ident,
                &load_context_var_ident,
            ) {
                Ok(def_transform) => def_transform,
                Err(e) => return e.to_compile_error().into(),
            };
            (Some(generated_def), def_type, def_transform)
        }
    };
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let (transformation, resolver_fns) = (def_transform.transformation, def_transform.resolver_fns);
    let resolver_fns = if resolver_fns.is_empty() {
        None
    } else {
        Some(quote! {
            impl #impl_generics #input_ident #ty_generics #where_clause {
                #(#resolver_fns)*
            }
        })
    };

    quote! {
        #generated_def

        impl #impl_generics #from_def_trait #ty_generics for #input_ident #where_clause {
            type Def = #def_type;

            fn from_def(
                #def_var_ident: Self::Def,
                #load_context_var_ident: &mut #bevy_asset_crate::LoadContext<'_>,
            ) -> std::result::Result<Self, #elf_crate_path::FromDefError> {
                Ok(#transformation)
            }
        }

        #resolver_fns
    }
    .into()
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

fn resolve_crate_name(orig_name: &str) -> syn::Result<proc_macro2::TokenStream> {
    match crate_name(orig_name) {
        Ok(FoundCrate::Itself) => Ok(quote!(crate)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(#ident))
        }
        Err(e) => Err(syn::Error::new(
            Span::call_site(),
            format!("could not resolve crate {orig_name} - {e}"),
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
