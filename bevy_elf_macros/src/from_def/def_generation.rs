use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    AngleBracketedGenericArguments, Attribute, Data, DataEnum, DataStruct, DeriveInput, Field,
    Fields, Generics, PathArguments, TypePath, Visibility, parse2, punctuated::Punctuated,
    token::Comma,
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
    let def_type_definition = match &derive_input.data {
        Data::Struct(input_struct) => generate_def_for_struct(
            input_struct,
            &derive_input.vis,
            def_type,
            &derive_input.generics,
        ),
        Data::Enum(input_enum) => generate_def_for_enum(
            input_enum,
            &derive_input.vis,
            def_type,
            &derive_input.generics,
        ),
        Data::Union(_) => Err(syn::Error::new_spanned(
            derive_input,
            "unions are not supported",
        )),
    }?;
    let serde = CratePath::try_from("serde")?;
    let attrs = if attrs.is_empty() {
        quote!(#[derive(#serde::Serialize, #serde::Deserialize)])
    } else {
        quote!(#(#attrs)*)
    };
    Ok(quote! {
        #attrs
        #def_type_definition
    })
}

fn generate_def_for_struct(
    input_struct: &DataStruct,
    vis: &Visibility,
    def_type: &syn::Type,
    generics: &Generics,
) -> Result<TokenStream, syn::Error> {
    let fields = match input_struct.fields.clone() {
        Fields::Named(mut named) => {
            named.named = generate_def_fields(named.named)?;
            Fields::Named(named)
        }
        Fields::Unnamed(mut unnamed) => {
            unnamed.unnamed = generate_def_fields(unnamed.unnamed)?;
            Fields::Unnamed(unnamed)
        }
        Fields::Unit => Fields::Unit,
    };
    let semi_token = input_struct.semi_token;
    Ok(quote! {
        #vis struct #def_type #generics #fields #semi_token
    })
}

fn generate_def_for_enum(
    input_enum: &DataEnum,
    vis: &Visibility,
    def_type: &syn::Type,
    generics: &Generics,
) -> Result<TokenStream, syn::Error> {
    let mut variants = input_enum.variants.clone();
    for variant in &mut variants {
        if let Some(VariantElfAttr(attrs)) = VariantElfAttr::from_attrs(&variant.attrs)? {
            variant.attrs = attrs;
        } else {
            variant.attrs.clear();
        };
        variant.fields = match variant.fields.clone() {
            Fields::Named(mut named) => {
                named.named = generate_def_fields(named.named)?;
                Fields::Named(named)
            }
            Fields::Unnamed(mut unnamed) => {
                unnamed.unnamed = generate_def_fields(unnamed.unnamed)?;
                Fields::Unnamed(unnamed)
            }
            Fields::Unit => Fields::Unit,
        };
    }
    let variants = variants.into_iter();
    Ok(quote! {
        #vis enum #def_type #generics {
            #(#variants,)*
        }
    })
}

fn generate_def_fields(fields: Punctuated<Field, Comma>) -> syn::Result<Punctuated<Field, Comma>> {
    fields
        .into_iter()
        .filter_map(|f| generate_def_field(f).transpose())
        .collect()
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
            let asset_module = CratePath::try_from(ELF_MODULE_PATH)?;
            quote!(#asset_module::FromDefWithResolver)
        }
        _ => {
            let asset_module = CratePath::try_from(ELF_MODULE_PATH)?;
            quote!(#asset_module::FromDef)
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
    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { .. }) =
        &last_segment.arguments
    else {
        return false;
    };
    last_segment.ident == "Handle" || last_segment.ident == "AssetRef"
}
