use std::fmt::Debug;

use proc_macro2::Ident;
use proc_macro2::Span;
use syn::{Attribute, Expr, LitStr, Token, parenthesized, parse::Parse, spanned::Spanned};

mod def_generation;
mod from_def_conversion;

pub use def_generation::generate_def_for;
pub use from_def_conversion::{DefTransformResult, generate_def_transform};

use crate::CratePath;
use crate::ELF_MODULE_PATH;

#[derive(Debug)]
pub struct FieldAttr {
    from_default: bool,
    default: bool,
    implicit: bool,
    spec: Option<FieldSpec>,
    resolver: Option<Expr>,
    expose_resolver: bool,
}

impl Parse for FieldAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut from_default = false;
        let mut default = false;
        let mut implicit = false;
        let mut spec: Option<FieldSpec> = None;
        let mut resolver: Option<Expr> = None;
        let mut expose_resolver = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "from_default" => from_default = true,
                "default" => default = true,
                "implicit" => implicit = true,
                "with_spec" => {
                    let spec_args;
                    parenthesized!(spec_args in input);
                    spec = Some(FieldSpec::parse(&spec_args)?);
                }
                "with_resolver" => {
                    let resolver_expr;
                    parenthesized!(resolver_expr in input);
                    resolver = Some(Expr::parse(&resolver_expr)?);
                }
                "expose_resolver" => expose_resolver = true,
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `from_default`, `default`, `implicit`, `with_spec`, `with_resolver` or `expose_resolver`",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            from_default,
            default,
            implicit,
            spec,
            resolver,
            expose_resolver,
        })
    }
}

impl FieldAttr {
    pub fn parse<'a>(attrs: impl IntoIterator<Item = &'a Attribute>) -> syn::Result<Option<Self>> {
        let mut result = None;
        for attr in attrs {
            if attr.path().is_ident("elf") {
                if result.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only one of `from_default`, `default` or `implicit` is allowed",
                    ));
                }
                let field_attr: Self = attr.parse_args()?;
                field_attr.validate(attr.path().span())?;
                result = Some(field_attr);
            }
        }
        Ok(result)
    }

    fn validate(&self, span: Span) -> syn::Result<()> {
        let Self {
            from_default,
            default,
            implicit,
            spec,
            resolver,
            expose_resolver,
        } = self;
        if !from_default
            && !default
            && !implicit
            && spec.is_none()
            && resolver.is_none()
            && !expose_resolver
        {
            Err(syn::Error::new(
                span,
                "expected at least one of `from_default`, `default`, `implicit`, `with_spec` or `with_resolver`",
            ))
        } else if *implicit && spec.as_ref().is_some_and(|spec| spec.extension.is_none()) {
            Err(syn::Error::new(
                span,
                "expected `extension` on implicit field",
            ))
        } else if spec.is_some() && resolver.is_some() {
            Err(syn::Error::new(
                span,
                "cannot use both `with_spec` and `with_resolver`",
            ))
        } else if (*from_default || *default) && (*implicit || spec.is_some() || resolver.is_some())
        {
            Err(syn::Error::new(
                span,
                "`from_default` and `default` cannot be combined with other parameters",
            ))
        } else {
            Ok(())
        }
    }

    pub fn omit_def_field(&self) -> bool {
        self.from_default || self.default || self.implicit
    }
}

pub struct FieldSpec {
    pub path_kind: PathKind,
    pub extension: Option<LitStr>,
}

impl Debug for FieldSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref extension) = self.extension {
            write!(
                f,
                "({:?}, extension = \"{}\")",
                self.path_kind,
                extension.value()
            )
        } else {
            write!(f, "({:?})", self.path_kind)
        }
    }
}

pub enum PathKind {
    Root(LitStr),
    Child(Option<LitStr>),
}

impl Debug for PathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root(base_path) => write!(f, "base_path = \"{}\"", base_path.value()),
            Self::Child(sub_path) => match sub_path {
                Some(sub_path) => write!(f, "sub_path = \"{}\"", sub_path.value()),
                None => write!(f, "sub_path"),
            },
        }
    }
}

impl Parse for FieldSpec {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut path_kind: Option<PathKind> = None;
        let mut extension: Option<LitStr> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "base_path" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    path_kind = Some(PathKind::Root(lit));
                }
                "sub_path" => {
                    let path = if input.peek(Token![=]) {
                        input.parse::<Token![=]>()?;
                        let lit: LitStr = input.parse()?;
                        Some(lit)
                    } else {
                        None
                    };
                    path_kind = Some(PathKind::Child(path));
                }
                "extension" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    extension = Some(lit);
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `base_path`, or `extension`",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if path_kind.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "either `base_path = \"base/path\"` or `sub_path [= \"sub/path\"]` is required",
            ));
        }

        Ok(Self {
            path_kind: path_kind.unwrap(),
            extension,
        })
    }
}

pub fn derive_def_type_name(asset_type_name: &str) -> String {
    let prefix = asset_type_name
        .strip_suffix("Asset")
        .unwrap_or(asset_type_name);
    format!("{prefix}Def")
}

pub fn from_def_trait() -> Result<CratePath, syn::Error> {
    let path = ELF_MODULE_PATH.to_string() + "::FromDef";
    CratePath::try_from(path.as_str())
}
