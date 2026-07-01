use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    AngleBracketedGenericArguments, Data, DataEnum, DataStruct, DeriveInput, Field, Fields,
    FieldsNamed, FieldsUnnamed, GenericArgument, Ident, PathArguments, Type, TypePath,
    spanned::Spanned,
};

use crate::{
    CratePath, ELF_MODULE_PATH,
    from_def::{FieldAttr, FieldSpec, PathKind},
};

struct FromDefImplContext {
    pub def_var_ident: TokenStream,
    pub load_context_var_ident: TokenStream,
}

impl FromDefImplContext {
    fn new(def_var_ident: impl ToTokens, load_context_var_ident: impl ToTokens) -> Self {
        Self {
            def_var_ident: def_var_ident.to_token_stream(),
            load_context_var_ident: load_context_var_ident.to_token_stream(),
        }
    }
}

pub struct DefTransformResult {
    pub transformation: TokenStream,
    pub resolver_fns: Vec<TokenStream>,
}

pub fn generate_def_transform(
    derive_input: &DeriveInput,
    def_type: &syn::Type,
    def_var_ident: impl ToTokens,
    load_context_var_ident: impl ToTokens,
) -> Result<DefTransformResult, syn::Error> {
    let ctx = FromDefImplContext::new(def_var_ident, load_context_var_ident);
    match &derive_input.data {
        Data::Struct(input_struct) => generate_def_transform_for_struct(input_struct, &ctx),
        Data::Enum(input_enum) => generate_def_transform_for_enum(input_enum, def_type, &ctx),
        Data::Union(_) => Err(syn::Error::new(
            derive_input.span(),
            "def to asset conversion generation not supported for unions",
        )),
    }
}

fn generate_def_transform_for_struct(
    input_struct: &DataStruct,
    ctx: &FromDefImplContext,
) -> Result<DefTransformResult, syn::Error> {
    Ok(match &input_struct.fields {
        Fields::Unit => DefTransformResult {
            transformation: quote!(Self),
            resolver_fns: Vec::new(),
        },
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            let def_var_ident = &ctx.def_var_ident;
            let FieldResults {
                field_conversions,
                resolver_fns,
                ..
            } = unnamed
                .iter()
                .enumerate()
                .map(|(field_index, field)| {
                    let field_idx = syn::Index::from(field_index);
                    let field_access = quote!(#def_var_ident.#field_idx);
                    let field_ident = Ident::new(&format!("field{field_index}"), field.span());
                    process_field(field, &field_ident, field_access, ctx)
                })
                .collect::<Result<FieldResults, syn::Error>>()?;
            DefTransformResult {
                transformation: quote!(Self( #(#field_conversions),* )),
                resolver_fns,
            }
        }
        Fields::Named(FieldsNamed { named, .. }) => {
            let def_var_ident = &ctx.def_var_ident;
            let FieldResults {
                field_conversions,
                resolver_fns,
                ..
            } = named
                .iter()
                .map(|field| {
                    let field_ident = &field.ident;
                    let field_access = quote!(#def_var_ident.#field_ident);
                    process_field(field, field_ident.as_ref().unwrap(), field_access, ctx)
                })
                .collect::<Result<FieldResults, syn::Error>>()?;
            DefTransformResult {
                transformation: quote!(Self { #(#field_conversions),* }),
                resolver_fns,
            }
        }
    })
}

fn generate_def_transform_for_enum(
    input_enum: &DataEnum,
    def_type: &Type,
    ctx: &FromDefImplContext,
) -> Result<DefTransformResult, syn::Error> {
    let mut variant_conversions = Vec::new();
    let mut resolver_fns = Vec::new();
    for variant in input_enum.variants.iter() {
        let variant_ident = &variant.ident;
        let (variant_conversion, mut variant_resolver_fns) = match &variant.fields {
            Fields::Unit => (
                quote!(#def_type::#variant_ident => Self::#variant_ident),
                Vec::new(),
            ),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let FieldResults {
                    def_fields,
                    field_conversions,
                    resolver_fns,
                } = unnamed
                    .iter()
                    .enumerate()
                    .map(|(field_index, field)| {
                        let ident = generate_field_name_for_unnamed(
                            Some(&variant_ident.to_string().to_case(Case::Snake)),
                            field_index,
                            field.span(),
                        );
                        (field, ident)
                    })
                    .map(|(field, ident)| process_field(field, &ident, &ident, ctx))
                    .collect::<Result<FieldResults, syn::Error>>()?;
                (
                    quote! {
                        #def_type::#variant_ident( #(#def_fields),* ) => Self::#variant_ident( #(#field_conversions),* )
                    },
                    resolver_fns,
                )
            }
            Fields::Named(FieldsNamed { named, .. }) => {
                let FieldResults {
                    def_fields,
                    field_conversions,
                    resolver_fns,
                } = named
                    .iter()
                    .map(|field| {
                        let field_ident = field.ident.as_ref();
                        process_field(field, field_ident.unwrap(), field_ident, ctx)
                    })
                    .collect::<Result<FieldResults, syn::Error>>()?;
                (
                    quote! {
                        #def_type::#variant_ident { #(#def_fields),* } => Self::#variant_ident { #(#field_conversions),* }
                    },
                    resolver_fns,
                )
            }
        };
        variant_conversions.push(variant_conversion);
        resolver_fns.append(&mut variant_resolver_fns);
    }

    let variant_conversions = variant_conversions.into_iter();
    let def_var_ident = &ctx.def_var_ident;
    Ok(DefTransformResult {
        transformation: quote! {
            match #def_var_ident {
                #(#variant_conversions),*
            }
        },
        resolver_fns,
    })
}

struct FieldResults {
    def_fields: Vec<TokenStream>,
    field_conversions: Vec<TokenStream>,
    resolver_fns: Vec<TokenStream>,
}

impl FromIterator<FieldResult> for FieldResults {
    fn from_iter<T: IntoIterator<Item = FieldResult>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut def_fields = Vec::new();
        let mut field_conversions = Vec::with_capacity(iter.size_hint().0);
        let mut resolver_fns = Vec::new();

        for FieldResult {
            def_field,
            def_conversion,
            resolver_fn,
        } in iter
        {
            field_conversions.push(def_conversion);
            if let Some(resolver_fn) = resolver_fn {
                resolver_fns.push(resolver_fn);
            }
            if let Some(def_field) = def_field {
                def_fields.push(def_field);
            }
        }

        Self {
            def_fields,
            field_conversions,
            resolver_fns,
        }
    }
}

struct FieldResult {
    def_field: Option<TokenStream>,
    def_conversion: TokenStream,
    resolver_fn: Option<TokenStream>,
}

fn process_field(
    field: &Field,
    artificial_field_ident: &Ident,
    field_access: impl ToTokens,
    ctx: &FromDefImplContext,
) -> Result<FieldResult, syn::Error> {
    let from_def_attr = FieldAttr::parse(&field.attrs)?;
    let resolver_expr = if let Some(field_spec) = from_def_attr.as_ref().and_then(|a| {
        if let FieldAttr::FromDef { spec, .. } = a {
            spec.as_ref()
        } else {
            None
        }
    }) {
        Some(generate_resolver_from(&field.ty, field_spec, ctx)?)
    } else {
        from_def_attr.as_ref().and_then(|a| {
            if let FieldAttr::FromDef { resolver, .. } = a {
                resolver.as_ref().map(|r| r.to_token_stream())
            } else {
                None
            }
        })
    };

    Ok(FieldResult {
        def_field: if from_def_attr
            .as_ref()
            .is_some_and(|attr| attr.omit_def_field())
        {
            None
        } else {
            Some(artificial_field_ident.to_token_stream())
        },
        def_conversion: generate_field_conversion(
            field,
            from_def_attr.as_ref(),
            resolver_expr.as_ref(),
            field_access,
            ctx,
        )?,
        resolver_fn: field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("expose_resolver"))
            .then(|| {
                generate_resolver_access(field, resolver_expr.as_ref(), artificial_field_ident)
            })
            .transpose()?,
    })
}

fn generate_field_conversion(
    field: &Field,
    from_def_attr: Option<&FieldAttr>,
    resolver_expr: Option<&TokenStream>,
    field_access: impl ToTokens,
    ctx: &FromDefImplContext,
) -> Result<TokenStream, syn::Error> {
    let asset_module = CratePath::try_from(ELF_MODULE_PATH)?;
    let colon = &field.colon_token;
    let field_type = &field.ty;
    let from_def_trait = from_def_trait()?;
    let field_ident = &field.ident;
    let ctx_var_ident = &ctx.load_context_var_ident;

    if let Some(FieldAttr::FromDef { default: true, .. }) = from_def_attr {
        return Ok(quote! {
            #field_ident #colon <#field_type as std::default::Default>::default()
        });
    }
    let def_expr = if let Some(FieldAttr::FromDefault) = &from_def_attr {
        quote! {
            <<#field_type as #asset_module::FromDef>::Def as std::default::Default>::default()
        }
    } else if let Some(FieldAttr::FromDef { implicit: true, .. }) = &from_def_attr {
        quote! {
            #asset_module::extract_id_from(#ctx_var_ident.path().clone())
        }
    } else {
        field_access.to_token_stream()
    };

    Ok(if let Some(resolver_expr) = resolver_expr {
        quote! {
            #field_ident #colon <#field_type as #asset_module::FromDefWithResolver>::from_def_with_resolver(
                #def_expr,
                &#resolver_expr,
                #ctx_var_ident
            )?
        }
    } else {
        quote! {
            #field_ident #colon <#field_type as #from_def_trait>::from_def(
                #def_expr,
                #ctx_var_ident
            )?
        }
    })
}

fn generate_resolver_access(
    field: &Field,
    resolver_expr: Option<&TokenStream>,
    artificial_field_ident: &Ident,
) -> Result<TokenStream, syn::Error> {
    let asset_module = CratePath::try_from(ELF_MODULE_PATH)?;
    let resolver_expr = if let Some(resolver_expr) = resolver_expr {
        resolver_expr
    } else {
        let asset_type = extract_asset_type(&field.ty).ok_or_else(|| syn::Error::new(
                field.ty.span(),
                "cannot `expose_resolver` for non-asset field - field must be of a type that contains a Handle",
            ))?;
        &quote! {
            <#asset_type as #asset_module::HasResolver>::resolver()
        }
    };

    let fn_name = generate_resolver_fn_name(artificial_field_ident);
    Ok(quote! {
        pub fn #fn_name() -> impl #asset_module::AssetResolver {
            #resolver_expr
        }
    })
}

fn generate_resolver_from(
    field_type: &syn::Type,
    spec: &FieldSpec,
    ctx: &FromDefImplContext,
) -> syn::Result<TokenStream> {
    let asset_module = CratePath::try_from(ELF_MODULE_PATH)?;
    let provider_expr = match &spec.path_kind {
        PathKind::Root(base_path) => {
            let extension = if let Some(extension) = spec.extension.as_ref() {
                quote!(Some(#extension))
            } else {
                quote!(None)
            };
            quote! {
                #asset_module::DynamicPathResolver {
                    base_path: #base_path.to_string(),
                    extension: #extension,
                }
            }
        }
        PathKind::Child(sub_path) => {
            let asset_type = extract_asset_type(field_type).ok_or_else(|| {
                syn::Error::new(
                    field_type.span(),
                    format!("`subpath` only allowed for types, that contain a bevy::asset::Handle"),
                )
            })?;
            let (sub_path, extension) = sub_path
                .as_ref()
                .map(|p| {
                    let extension = if let Some(extension) = spec.extension.as_ref() {
                        quote!(Some(#extension))
                    } else {
                        quote!(None)
                    };
                    (p.to_token_stream(), extension)
                })
                .unwrap_or_else(|| {
                    (
                        quote! {
                            <#asset_type as #asset_module::HasSpecProvider>::provider().base_path()
                        },
                        quote! {
                            <#asset_type as #asset_module::HasSpecProvider>::provider().extension()
                        },
                    )
                });
            let ctx_var_ident = &ctx.load_context_var_ident;
            quote! {
                #asset_module::DynamicPathResolver::resolve_sub_path(
                    #ctx_var_ident,
                    #sub_path,
                    #extension
                )?
            }
        }
    };

    Ok(provider_expr)
}

fn extract_asset_type(field_type: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(TypePath { path, .. }) = field_type else {
        return None;
    };
    let last_segment = path.segments.last()?;
    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &last_segment.arguments
    else {
        return None;
    };
    if last_segment.ident == "Handle" || last_segment.ident == "AssetRef" {
        return if let GenericArgument::Type(asset_type) = args.first()? {
            Some(asset_type)
        } else {
            None
        };
    } else {
        for generic_arg in args {
            if let GenericArgument::Type(inner) = generic_arg {
                let result = extract_asset_type(inner);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }
}

fn generate_field_name_for_unnamed(
    prefix: Option<&str>,
    field_index: usize,
    field_span: Span,
) -> Ident {
    let name = if let Some(prefix) = prefix {
        format!("{prefix}{field_index}")
    } else {
        format!("field{field_index}")
    };
    Ident::new(&name, field_span)
}

fn generate_resolver_fn_name(field_ident: &Ident) -> Ident {
    Ident::new(&format!("{field_ident}_resolver"), Span::call_site())
}

pub fn from_def_trait() -> Result<CratePath, syn::Error> {
    let path = ELF_MODULE_PATH.to_string() + "::FromDef";
    CratePath::try_from(path.as_str())
}
