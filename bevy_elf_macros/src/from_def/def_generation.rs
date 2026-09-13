use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    AngleBracketedGenericArguments, Attribute, DeriveInput, Field, PathArguments, TypePath, parse2,
};

use crate::{
    CratePath, ELF_MODULE_PATH,
    from_def::{FieldElfAttr, VariantElfAttr},
};

pub fn generate_def_for(
    derive_input: &DeriveInput,
    def_type: &syn::Type,
    attrs: &[Attribute],
) -> Result<TokenStream, syn::Error> {
    let serde = CratePath::try_from("serde")?;
    let attrs = if attrs.is_empty() {
        quote!(#[derive(#serde::Serialize, #serde::Deserialize)])
    } else {
        quote!(#(#attrs)*)
    };

    elf_macro_utils::generate_mirror_type(
        derive_input,
        def_type,
        Some(attrs),
        generate_def_field,
        |attrs| match VariantElfAttr::from_attrs(attrs)? {
            Some(VariantElfAttr(attrs)) => Ok(attrs),
            None => Ok(Vec::new()),
        },
    )
}

fn generate_def_field(mut field: Field) -> syn::Result<Option<Field>> {
    let elf = FieldElfAttr::from_attrs(&field.attrs)?;
    let from_def_trait = match elf {
        Some(attr) if attr.omit_def_field() => {
            if attr.implicit && !is_handle_or_asset_ref(&field.ty) {
                return Err(syn::Error::new_spanned(
                    field.ty,
                    "`implicit` can only be used for fields of type `Handle` or `AssetRef`",
                ));
            }
            return Ok(None);
        }
        Some(FieldElfAttr { spec: Some(_), .. })
        | Some(FieldElfAttr {
            resolver: Some(_), ..
        }) => {
            let elf_module = CratePath::try_from(ELF_MODULE_PATH)?;
            quote!(#elf_module::FromDefWithResolver)
        }
        _ => {
            let elf_module = CratePath::try_from(ELF_MODULE_PATH)?;
            quote!(#elf_module::FromDef)
        }
    };
    let field_type = &field.ty;
    if let Some(elf) = elf {
        field.attrs = elf.def_attrs;
    } else {
        field.attrs.clear();
    }
    field.ty = parse2(quote!(<#field_type as #from_def_trait>::Def))?;
    Ok(Some(field))
}

fn is_handle_or_asset_ref(ty: &syn::Type) -> bool {
    let syn::Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };

    let Some(last_segment) = path.segments.last() else {
        return false;
    };

    if matches!(
        last_segment.arguments,
        PathArguments::AngleBracketed(AngleBracketedGenericArguments { .. }),
    ) {
        last_segment.ident == "Handle" || last_segment.ident == "AssetRef"
    } else {
        false
    }
}
