use elf_macro_utils::{FieldTransform, generate_transform};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    AngleBracketedGenericArguments, DeriveInput, Field, GenericArgument, Ident, PathArguments,
    TypePath, spanned::Spanned,
};

use crate::{
    CratePath, ELF_MODULE_PATH,
    from_def::{FieldElfAttr, FieldSpec, PathKind},
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
) -> syn::Result<DefTransformResult> {
    let ctx = FromDefImplContext::new(def_var_ident, load_context_var_ident);
    let result = generate_transform(
        derive_input,
        def_type,
        quote!(Self),
        &ctx.def_var_ident,
        |field, artificial_field_ident, field_access| {
            process_field(field, artificial_field_ident, field_access, &ctx)
        },
    )?;

    Ok(DefTransformResult {
        transformation: result.transformation,
        resolver_fns: result.extra_items,
    })
}

fn process_field(
    field: &Field,
    artificial_field_ident: &Ident,
    field_access: impl ToTokens,
    ctx: &FromDefImplContext,
) -> Result<FieldTransform, syn::Error> {
    let elf_attr = FieldElfAttr::from_attrs(&field.attrs)?;
    let resolver_expr = if let Some(field_spec) = elf_attr.as_ref().and_then(|a| a.spec.as_ref()) {
        Some(generate_resolver_from(&field.ty, field_spec, ctx)?)
    } else {
        elf_attr
            .as_ref()
            .and_then(|a| a.resolver.as_ref().map(|r| r.to_token_stream()))
    };

    Ok(FieldTransform {
        bound_ident: if elf_attr.as_ref().is_some_and(|attr| attr.omit_def_field()) {
            None
        } else {
            Some(artificial_field_ident.to_token_stream())
        },
        conversion_expr: generate_field_conversion(
            field,
            elf_attr.as_ref(),
            resolver_expr.as_ref(),
            field_access,
            ctx,
        )?,
        extra_items: elf_attr
            .and_then(|elf| {
                elf.expose_resolver.then(|| {
                    generate_resolver_access(field, resolver_expr.as_ref(), artificial_field_ident)
                })
            })
            .transpose()?
            .map_or_else(Vec::default, |resolver_access| vec![resolver_access]),
    })
}

fn generate_field_conversion(
    field: &Field,
    from_def_attr: Option<&FieldElfAttr>,
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

    if let Some(FieldElfAttr { default: true, .. }) = from_def_attr {
        return Ok(quote! {
            #field_ident #colon <#field_type as std::default::Default>::default()
        });
    }
    let def_expr = if let Some(FieldElfAttr {
        from_default: true, ..
    }) = &from_def_attr
    {
        quote! {
            <<#field_type as #asset_module::FromDef>::Def as std::default::Default>::default()
        }
    } else if let Some(FieldElfAttr { implicit: true, .. }) = &from_def_attr {
        quote! {
            #asset_module::extract_id_from(#ctx_var_ident.path().clone())?
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
                    "`sub_path` only allowed for types, that contain a `bevy_asset::Handle`"
                        .to_owned(),
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
        if let GenericArgument::Type(asset_type) = args.first()? {
            Some(asset_type)
        } else {
            None
        }
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

fn generate_resolver_fn_name(field_ident: &Ident) -> Ident {
    Ident::new(&format!("{field_ident}_resolver"), Span::call_site())
}

pub fn from_def_trait() -> Result<CratePath, syn::Error> {
    let path = ELF_MODULE_PATH.to_string() + "::FromDef";
    CratePath::try_from(path.as_str())
}
