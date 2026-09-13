use elf_macro_utils::{CratePath, FieldTransform};
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{DeriveInput, Ident, TypePath, parse2};

const ENTITY_LOOKUP_MODULE_PATH: &str = "bevy_entity_lookup";

pub fn into_looked_up_impl(derive_input: &DeriveInput) -> syn::Result<TokenStream> {
    let input_ident = &derive_input.ident;
    let looked_up_type_name = derive_looked_up_type_name(&input_ident.to_string());
    let looked_up_type = syn::parse_str(&looked_up_type_name)?;
    let lookup_map_var_ident = Ident::new("lookup_map", Span::call_site());
    let into_looked_up_trait = into_looked_up_trait()?;

    let generated_looked_up = elf_macro_utils::generate_mirror_type(
        derive_input,
        &looked_up_type,
        Some(quote!(#[derive(std::clone::Clone)])),
        |mut field| {
            field.attrs.clear();
            let field_type = &field.ty;
            field.ty = parse2(quote!(<#field_type as #into_looked_up_trait>::LookedUp))?;
            if is_entity_id(&field.ty) {
                field.ty = syn::parse2(entity_type_path()?)?;
            }
            Ok(Some(field))
        },
        |_| Ok(Vec::default()),
    )?;

    let transformation = elf_macro_utils::generate_transform(
        derive_input,
        quote!(Self),
        &looked_up_type,
        quote!(self),
        |field, artificial_field_ident, field_access| {
            let field_type = &field.ty;
            let colon = field.colon_token;
            let field_ident = &field.ident;

            Ok(FieldTransform {
                bound_ident: Some(artificial_field_ident.to_token_stream()),
                conversion_expr: quote!{
                    #field_ident #colon <#field_type as #into_looked_up_trait>::into_looked_up(#field_access, #lookup_map_var_ident)?
                },
                extra_items: Vec::default(),
            })
        },
    )?
    .transformation;

    let (impl_generics, ty_generics, where_clause) = derive_input.generics.split_for_impl();
    let lookup_crate = CratePath::try_from("bevy_entity_lookup")?;

    Ok(quote! {
        #generated_looked_up

        impl #impl_generics #into_looked_up_trait #ty_generics for #input_ident #where_clause {
            type LookedUp = #looked_up_type;

            fn into_looked_up(
                &self,
                #lookup_map_var_ident: &#lookup_crate::LookupMap
            ) -> std::result::Result<Self::LookedUp, #lookup_crate::MissingEntity> {
                Ok(#transformation)
            }
        }
    })
}

fn is_entity_id(ty: &syn::Type) -> bool {
    let syn::Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };

    let Some(last_segment) = path.segments.last() else {
        return false;
    };

    if last_segment.arguments.is_none() {
        last_segment.ident == "EntityId"
    } else {
        false
    }
}

fn entity_type_path() -> syn::Result<proc_macro2::TokenStream> {
    if let Ok(found) = crate_name("bevy_ecs") {
        return Ok(match found {
            FoundCrate::Itself => quote!(crate::entity::Entity),
            FoundCrate::Name(name) => {
                let ident = Ident::new(&name, Span::call_site());
                quote!(#ident::entity::Entity)
            }
        });
    }
    match crate_name("bevy") {
        Ok(FoundCrate::Itself) => Ok(quote!(crate::ecs::entity::Entity)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(#ident::asset))
        }
        Err(e) => Err(syn::Error::new(
            Span::call_site(),
            format!("could not resolve `bevy_ecs` or `bevy` (needed for `Entity`): {e}"),
        )),
    }
}

fn derive_looked_up_type_name(asset_type_name: &str) -> String {
    let prefix = asset_type_name
        .strip_suffix("Asset")
        .unwrap_or(asset_type_name);
    format!("{prefix}LookedUp")
}

fn into_looked_up_trait() -> Result<CratePath, syn::Error> {
    let path = ENTITY_LOOKUP_MODULE_PATH.to_string() + "::IntoLookedUp";
    CratePath::try_from(path.as_str())
}
