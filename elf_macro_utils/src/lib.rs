use std::ops;

// bevy_macro_mirror/src/lib.rs
use proc_macro_crate::{FoundCrate, crate_name};
use quote::ToTokens;
use syn::{Ident, spanned::Spanned};

pub use mirror_generation::generate_mirror_type;
pub use mirror_transform::*;

mod mirror_generation;
mod mirror_transform;

/// A [`syn::Path`] whose first segment is rewritten to the correct crate name.
///
/// Resolution happens in [`CratePath::try_from`] using
/// [`proc_macro_crate::crate_name`].
#[derive(Clone)]
pub struct CratePath(syn::Path);

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
