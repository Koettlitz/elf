//! This trait provides macros, that go along with the bevy_elf crate.
//! It is recommended to use `bevy_elf`, which re-exports this crates macros instead of using this
//! crate directly.

use std::ops;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    DeriveInput, Ident, Item, ItemEnum, ItemStruct, Token, Type, TypePath, parse_macro_input,
    punctuated::Punctuated, spanned::Spanned,
};

use crate::{
    from_def::{
        DefTransformResult, derive_def_type_name, from_def_trait, generate_def_for,
        generate_def_transform,
    },
    spec::{SpecArgs, create_spec_impl},
};

mod from_def;
mod spec;

const ELF_MODULE_PATH: &'static str = "bevy_elf";

/// Generates an implementation of `AssetPathSpec` and `HasResolver` for the annotated struct or
/// enum. This enables this asset to be resolved from a file name prefix when used as a field
/// of a type implementing `FromDef`.
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
                "asset_spec attribute is only valid for structs and enums",
            )
            .to_compile_error();
            return quote! {
                #item
                #error
            }
            .into();
        }
    };
    let args = parse_macro_input!(attr as SpecArgs);
    let spec_impl = match create_spec_impl(type_ident, &args) {
        Ok(resolver_impl) => resolver_impl,
        Err(e) => e.to_compile_error(),
    };
    let asset_module = match CratePath::try_from(ELF_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.to_compile_error().into(),
    };
    quote! {
        #item

        #spec_impl

        impl #asset_module::HasResolver for #type_ident {
            type Resolver = #asset_module::ResolverSpec<Self>;

            fn resolver() -> Self::Resolver {
                #asset_module::ResolverSpec::<Self>::default()
            }
        }
    }
    .into()
}

/// Implements the trait `FromDef` for the annotated struct or enum.
/// The def type (`FromDef::Def`) can be provided by the additional attribute
/// `#[def_type(DefType)]`.
///
/// There are the following ways to specify the def_type:
///     1. `#[def_type(Self)]` where no conversion is necessary, because the serializable type is
///        also the runtime type. `from_def()` just returns `Self` as is.
///     2. `#[def_type(CustomType)]` to provide a custom serializable def type to be used.
///        That type needs to have a corresponding field with the same name for each field in `Self`
///        that should be converted and each such field must implement FromDef.
///        The field types must match the corresponding field's type in `Self` in terms
///        of its `FromDef::Def` type.
///     3. `#[def_type(())]` - use this when the type has no fields that need serialization.
///     4. If the additional `#[def_type]` attribute is omitted this macro generates a
///        def type.
/// All primitive types, container types like [`Option`], [`Vec`] and
/// [`HashMap`](std::collections::HashMap), as well as `Handle` and `AssetRef` implement `FromDef`.
///
/// It is possible to influence resolution and def type generation by using the
/// `#[elf(...)]` attribute on the fields directly:
///
/// `#[elf(default)]` will use the [`Default`](`std::default::Default`) trait to construct a value, so
/// it omits the field in the generated def type and also skips resolution completely.
///
/// `#[elf(from_default)]` will use the [`Default`](`std::default::Default`) trait to construct the value of the
/// field's def type, so it omits the field in the generated def type and passes the default value
/// to the field types `from_def` method.
///
/// `#[elf(implicit)]` will omit the field in the generated def type and use the same id
/// as the parent (containing asset) to resolve the file name.
/// The `implicit` option can be combined freely with `with_spec`.
///
/// `#[elf(with_spec(base_path = "base/path"))]` overrides the `base_path` of the field's
/// type used for resolution. This is only relevant for types like `bevy::asset::Handle<T>` or `AssetRef<T>`.
///
/// Alternatively to specifying a `base_path` you can use
/// `#[elf(with_spec(sub_path = "foo"))]` to make the field being resolved relatively
/// to the current path (the `base_path` used to resolve the containing type).
///
/// `#[elf(with_resolver(CustomResolver))]` can be used to specify a custom type that implements
/// `AssetResolver`, which is used to resolve the asset path from the string id.
///
/// Use `#[elf(expose_resolver)]` on a field to generate a function on the type containing the field
/// which exposes the resolver. The name is derived from the field name (e.g.
/// `MyAsset::foo_resolver()` for the field `foo`).
///
/// # Example
/// ```ignore
/// #[derive(FromDef, Asset, TypePath)]
/// struct Spritesheet {
///     #[elf(implicit, with_spec(sub_path = "image", extension = "png"))]
///     image: Handle<Image>,
///
///     #[elf(implicit, with_spec(sub_path = "layout", extension = "tl.ron"))]
///     spritesheet_layout: Handle<TextureAtlasLayout>,
///     kind: SpritesheetKind,
/// }
///
/// #[derive(FromDef)]
/// enum SpritesheetKind {
///     Static(usize),
///     Animated(
///         #[elf(with_spec(base_path = "animations", extension = "ani.ron"))]
///         Handle<SpriteAnimationAsset>,
///     ),
/// }
/// ```
#[proc_macro_derive(FromDef, attributes(def_type, elf))]
pub fn from_def(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let asset_module = match CratePath::try_from(ELF_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.into_compile_error().into(),
    };
    let bevy_crate = match resolve_crate_name("bevy") {
        Ok(bevy_crate) => bevy_crate,
        Err(e) => return e.into_compile_error().into(),
    };
    let load_context_var_ident = Ident::new("ctx", Span::call_site());
    let input_ident = &input.ident;
    let from_def_trait = match from_def_trait() {
        Ok(from_def_trait) => from_def_trait,
        Err(e) => return e.into_compile_error().into(),
    };
    let def_var_ident = Ident::new("def", Span::call_site());

    let mut def_type: Option<syn::Type> = None;
    for attribute in &input.attrs {
        if attribute.path().is_ident("def_type") {
            def_type = match attribute.parse_args() {
                Ok(ty) => Some(ty),
                Err(e) => return e.to_compile_error().into(),
            };
        }
    }
    let (generated_def, def_type, def_transform) = match def_type {
        None => {
            let def_type_name = derive_def_type_name(&input_ident.to_string());
            let def_type = match syn::parse_str(&def_type_name) {
                Ok(def_type) => def_type,
                Err(e) => return e.to_compile_error().into(),
            };
            let generated_def = match generate_def_for(&input, &def_type) {
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
        Some(def_type) if is_self(&def_type) => (
            None,
            def_type,
            DefTransformResult {
                transformation: def_var_ident.to_token_stream(),
                resolver_fns: Vec::new(),
            },
        ),
        Some(def_type) => {
            let def_transform = match generate_def_transform(
                &input,
                &def_type,
                &def_var_ident,
                &load_context_var_ident,
            ) {
                Ok(cimpl) => cimpl,
                Err(e) => return e.to_compile_error().into(),
            };
            (None, def_type, def_transform)
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
            type Error = #asset_module::FromDefError;

            fn from_def(
                #def_var_ident: Self::Def,
                #load_context_var_ident: &mut #bevy_crate::asset::LoadContext<'_>,
            ) -> std::result::Result<Self, Self::Error> {
                Ok(#transformation)
            }
        }

        #resolver_fns
    }
    .into()
}

#[proc_macro]
pub fn from_def_self(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated<Type, Token![,]>::parse_terminated);
    let bevy_crate = match resolve_crate_name("bevy") {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };
    let from_def_trait = match from_def_trait() {
        Ok(from_def_trait) => from_def_trait,
        Err(e) => return e.into_compile_error().into(),
    };
    let asset_module = match CratePath::try_from(ELF_MODULE_PATH) {
        Ok(asset_module) => asset_module,
        Err(e) => return e.into_compile_error().into(),
    };
    let mut impls = Vec::with_capacity(input.len());
    for ident in input {
        let impl_block = quote! {
            impl #from_def_trait for #ident {
                type Def = Self;
                type Error = #asset_module::FromDefError;

                fn from_def(
                    def: Self::Def,
                    _: &mut #bevy_crate::asset::LoadContext<'_>,
                ) -> Result<Self, Self::Error> {
                    Ok(def)
                }
            }
        };
        impls.push(impl_block);
    }

    quote! {
        #(#impls)*
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
        syn::Type::Path(TypePath { qself: None, path }) => path.is_ident("Self"),
        _ => false,
    }
}
