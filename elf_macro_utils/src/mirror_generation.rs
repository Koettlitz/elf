use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Field, Fields, Generics, Visibility,
    punctuated::Punctuated, token::Comma,
};

/// Generates a "mirror" struct or enum: same shape (visibility, generics,
/// field/variant names, struct/enum-ness) as `derive_input`, but with each
/// field transformed by `mirror_field` and — for enums — each variant's
/// attributes transformed by `mirror_variant_attrs`.
///
/// `mirror_field` receives each field of the original type and returns:
/// - `Ok(Some(field))` to include a (possibly retyped/reattributed) field
///   in the mirror type,
/// - `Ok(None)` to omit the field from the mirror type entirely,
/// - `Err(_)` to abort with a compile error.
pub fn generate_mirror_type(
    derive_input: &DeriveInput,
    mirror_type: &syn::Type,
    derive_attrs: Option<TokenStream>,
    mut mirror_field: impl FnMut(Field) -> syn::Result<Option<Field>>,
    mut mirror_variant_attrs: impl FnMut(&[Attribute]) -> syn::Result<Vec<Attribute>>,
) -> syn::Result<TokenStream> {
    let type_definition = match &derive_input.data {
        Data::Struct(input_struct) => generate_mirror_struct(
            input_struct,
            &derive_input.vis,
            mirror_type,
            &derive_input.generics,
            &mut mirror_field,
        ),
        Data::Enum(input_enum) => generate_mirror_enum(
            input_enum,
            &derive_input.vis,
            mirror_type,
            &derive_input.generics,
            &mut mirror_field,
            &mut mirror_variant_attrs,
        ),
        Data::Union(_) => Err(syn::Error::new_spanned(
            derive_input,
            "unions are not supported",
        )),
    }?;
    Ok(quote! {
        #derive_attrs
        #type_definition
    })
}

fn generate_mirror_struct(
    input_struct: &DataStruct,
    vis: &Visibility,
    mirror_type: &syn::Type,
    generics: &Generics,
    mirror_field: &mut impl FnMut(Field) -> syn::Result<Option<Field>>,
) -> syn::Result<TokenStream> {
    let fields = match input_struct.fields.clone() {
        Fields::Named(mut named) => {
            named.named = generate_mirror_fields(named.named, mirror_field)?;
            Fields::Named(named)
        }
        Fields::Unnamed(mut unnamed) => {
            unnamed.unnamed = generate_mirror_fields(unnamed.unnamed, mirror_field)?;
            Fields::Unnamed(unnamed)
        }
        Fields::Unit => Fields::Unit,
    };
    let semi_token = input_struct.semi_token;
    Ok(quote! {
        #vis struct #mirror_type #generics #fields #semi_token
    })
}

fn generate_mirror_enum(
    input_enum: &DataEnum,
    vis: &Visibility,
    mirror_type: &syn::Type,
    generics: &Generics,
    mirror_field: &mut impl FnMut(Field) -> syn::Result<Option<Field>>,
    mirror_variant_attrs: &mut impl FnMut(&[Attribute]) -> syn::Result<Vec<Attribute>>,
) -> syn::Result<TokenStream> {
    let mut variants = input_enum.variants.clone();
    for variant in &mut variants {
        variant.attrs = mirror_variant_attrs(&variant.attrs)?;
        variant.fields = match variant.fields.clone() {
            Fields::Named(mut named) => {
                named.named = generate_mirror_fields(named.named, mirror_field)?;
                Fields::Named(named)
            }
            Fields::Unnamed(mut unnamed) => {
                unnamed.unnamed = generate_mirror_fields(unnamed.unnamed, mirror_field)?;
                Fields::Unnamed(unnamed)
            }
            Fields::Unit => Fields::Unit,
        };
    }
    let variants = variants.into_iter();
    Ok(quote! {
        #vis enum #mirror_type #generics {
            #(#variants,)*
        }
    })
}

fn generate_mirror_fields(
    fields: Punctuated<Field, Comma>,
    mirror_field: &mut impl FnMut(Field) -> syn::Result<Option<Field>>,
) -> syn::Result<Punctuated<Field, Comma>> {
    fields
        .into_iter()
        .filter_map(|f| mirror_field(f).transpose())
        .collect()
}
