use std::fmt::Debug;

use proc_macro2::Ident;
use proc_macro2::Span;
use syn::Type;
use syn::{Attribute, Expr, LitStr, Token, parenthesized, parse::Parse, spanned::Spanned};

mod def_generation;
mod from_def_conversion;

pub use def_generation::generate_def_for;
pub use from_def_conversion::{DefTransformResult, generate_def_transform};

use crate::CratePath;
use crate::ELF_MODULE_PATH;

#[derive(Debug)]
pub enum TypeElfAttr {
    DefType(Box<Type>),
    DefAttrs(Vec<Attribute>),
}

impl TypeElfAttr {
    pub fn from_attrs<'a>(
        attrs: impl IntoIterator<Item = &'a Attribute>,
    ) -> syn::Result<Option<Self>> {
        let mut result = None;

        for attr in attrs {
            if attr.path().is_ident("elf") {
                if result.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only one `elf` attribute per type is allowed",
                    ));
                }
                let elf: Self = attr.parse_args()?;
                result = Some(elf);
            }
        }

        Ok(result)
    }
}

impl Parse for TypeElfAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut def_type: Option<Type> = None;
        let mut def_attrs: Vec<Attribute> = Vec::new();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "def_type" => {
                    if def_type.is_some() {
                        return Err(syn::Error::new(
                            ident.span(),
                            "Only one `def_type` argument is allowed.",
                        ));
                    }
                    if !def_attrs.is_empty() {
                        return Err(syn::Error::new(
                            ident.span(),
                            "Only one of `def_type` and `on_def` is allowed.",
                        ));
                    }
                    let buf;
                    parenthesized!(buf in input);
                    def_type = Some(Type::parse(&buf)?)
                }
                "on_def" => {
                    if !def_attrs.is_empty() {
                        return Err(syn::Error::new(
                            ident.span(),
                            "Only one `on_def` argument is allowed.",
                        ));
                    }
                    if def_type.is_some() {
                        return Err(syn::Error::new(
                            ident.span(),
                            "Only one of `def_type` and `on_def` is allowed.",
                        ));
                    }
                    let buf;
                    parenthesized!(buf in input);
                    def_attrs = Attribute::parse_outer(&buf)?;
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("Unknown parameter `{other}`. Expected `def_type` or `on_def`."),
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if let Some(def_type) = def_type {
            Ok(Self::DefType(Box::new(def_type)))
        } else if !def_attrs.is_empty() {
            Ok(Self::DefAttrs(def_attrs))
        } else {
            Err(syn::Error::new(
                Span::call_site(),
                "Empty `elf` attribute not allowed. Expected one of `def_type` or `on_def`.",
            ))
        }
    }
}

#[derive(Debug)]
pub struct FieldElfAttr {
    from_default: bool,
    default: bool,
    implicit: bool,
    spec: Option<FieldSpec>,
    resolver: Option<Expr>,
    def_attrs: Vec<Attribute>,
    expose_resolver: bool,
}

impl FieldElfAttr {
    pub fn from_attrs<'a>(
        attrs: impl IntoIterator<Item = &'a Attribute>,
    ) -> syn::Result<Option<Self>> {
        let mut result = None;
        for attr in attrs {
            if attr.path().is_ident("elf") {
                if result.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only one `elf` attribute per field is allowed",
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
            def_attrs,
            expose_resolver,
        } = self;
        if !from_default
            && !default
            && !implicit
            && spec.is_none()
            && resolver.is_none()
            && def_attrs.is_empty()
            && !expose_resolver
        {
            Err(syn::Error::new(
                span,
                "expected at least one of `from_default`, `default`, `implicit`, `with_spec`, `on_def` or `with_resolver`",
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
        } else if self.omit_def_field() && !def_attrs.is_empty() {
            Err(syn::Error::new(
                span,
                "`on_def` is not allowed when the field doesn't exist on the def type, due to `implicit`, `from_default` or `default`",
            ))
        } else {
            Ok(())
        }
    }

    pub fn omit_def_field(&self) -> bool {
        self.from_default || self.default || self.implicit
    }
}

impl Parse for FieldElfAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut from_default = false;
        let mut default = false;
        let mut implicit = false;
        let mut spec: Option<FieldSpec> = None;
        let mut resolver: Option<Expr> = None;
        let mut def_attrs: Vec<Attribute> = Vec::new();
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
                "on_def" => {
                    let buf;
                    parenthesized!(buf in input);
                    def_attrs = Attribute::parse_outer(&buf)?;
                }
                "expose_resolver" => expose_resolver = true,
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "Unknown parameter `{other}`. Expected `from_default`, `default`, `implicit`, `with_spec`, `with_resolver`, `on_def` or `expose_resolver`"
                        ),
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
            def_attrs,
            expose_resolver,
        })
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

pub struct VariantElfAttr(pub Vec<Attribute>);

impl VariantElfAttr {
    pub fn from_attrs<'a>(
        attrs: impl IntoIterator<Item = &'a Attribute>,
    ) -> syn::Result<Option<Self>> {
        let mut result = None;

        for attr in attrs {
            if attr.path().is_ident("elf") {
                if result.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only one `elf` attribute per variant is allowed",
                    ));
                }
                let elf: Self = attr.parse_args()?;
                result = Some(elf);
            }
        }

        Ok(result)
    }
}

impl Parse for VariantElfAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut def_attrs: Vec<Attribute> = Vec::new();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "on_def" => {
                    if !def_attrs.is_empty() {
                        return Err(syn::Error::new(
                            ident.span(),
                            "Only one `on_def` argument is allowed.",
                        ));
                    }
                    let buf;
                    parenthesized!(buf in input);
                    def_attrs = Attribute::parse_outer(&buf)?;
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("Unknown parameter `{other}`. Expected `on_def`."),
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if !def_attrs.is_empty() {
            Ok(Self(def_attrs))
        } else {
            Err(syn::Error::new(
                Span::call_site(),
                "Empty `elf` attribute not allowed. Expected `on_def(#[...])`.",
            ))
        }
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
