// elf-macro-utils/src/transform.rs
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Field, Fields, FieldsNamed, FieldsUnnamed, Ident,
    spanned::Spanned,
};

/// The result of transforming a single field from the source value into the
/// corresponding piece of the target value.
pub struct FieldTransform {
    /// The identifier this field is bound to in an enum match pattern.
    /// Ignored for structs (which access fields directly via
    /// `source_var.field`, no pattern needed). `None` if the field isn't
    /// read from the source pattern at all (e.g. `Default::default()`).
    pub bound_ident: Option<TokenStream>,
    /// The `field_ident: <expr>` (bare `<expr>` for tuple fields) to place
    /// in the target value's constructor.
    pub conversion_expr: TokenStream,
    /// Extra top-level items this field contributes alongside the
    /// surrounding impl (e.g. companion functions). Most fields return none.
    pub extra_items: Vec<TokenStream>,
}

pub struct TransformResult {
    pub transformation: TokenStream,
    pub extra_items: Vec<TokenStream>,
}

/// Generates the body of a conversion function that turns a source value
/// into a target value with the same struct/enum shape.
///
/// - `pattern_type` is the type path used to match source enum variants
///   (e.g. `FooDef` for `FooDef::Variant(..) => ..`); unused for structs.
/// - `construct_type` is the type path used to construct the result
///   (`Self` in one direction, a separately named mirror type in the other).
/// - `source_var` is the expression holding the source value; struct fields
///   are accessed as `#source_var.#field`.
/// - `transform_field` is called once per field with the field itself, an
///   artificial identifier for it (stable even for tuple fields/enum
///   variants), and the expression that accesses it on the source value.
pub fn generate_transform(
    derive_input: &DeriveInput,
    pattern_type: impl ToTokens,
    construct_type: impl ToTokens,
    source_var: impl ToTokens,
    mut transform_field: impl FnMut(&Field, &Ident, TokenStream) -> syn::Result<FieldTransform>,
) -> syn::Result<TransformResult> {
    match &derive_input.data {
        Data::Struct(input_struct) => generate_transform_for_struct(
            input_struct,
            construct_type,
            source_var,
            &mut transform_field,
        ),
        Data::Enum(input_enum) => generate_transform_for_enum(
            input_enum,
            pattern_type,
            construct_type,
            source_var,
            &mut transform_field,
        ),
        Data::Union(_) => Err(syn::Error::new(
            derive_input.span(),
            "conversion generation not supported for unions",
        )),
    }
}

fn generate_transform_for_struct(
    input_struct: &DataStruct,
    construct_type: impl ToTokens,
    source_var: impl ToTokens,
    transform_field: &mut impl FnMut(&Field, &Ident, TokenStream) -> syn::Result<FieldTransform>,
) -> syn::Result<TransformResult> {
    Ok(match &input_struct.fields {
        Fields::Unit => TransformResult {
            transformation: quote!(#construct_type),
            extra_items: Vec::new(),
        },
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            let FieldResults {
                field_conversions,
                extra_items,
                ..
            } = unnamed
                .iter()
                .enumerate()
                .map(|(field_index, field)| {
                    let field_idx = syn::Index::from(field_index);
                    let field_access = quote!(#source_var.#field_idx);
                    let field_ident = Ident::new(&format!("field{field_index}"), field.span());
                    transform_field(field, &field_ident, field_access)
                })
                .collect::<syn::Result<FieldResults>>()?;
            TransformResult {
                transformation: quote!(#construct_type( #(#field_conversions),* )),
                extra_items,
            }
        }
        Fields::Named(FieldsNamed { named, .. }) => {
            let FieldResults {
                field_conversions,
                extra_items,
                ..
            } = named
                .iter()
                .map(|field| {
                    let field_ident = field.ident.as_ref().unwrap();
                    let field_access = quote!(#source_var.#field_ident);
                    transform_field(field, field_ident, field_access)
                })
                .collect::<syn::Result<FieldResults>>()?;
            TransformResult {
                transformation: quote!(#construct_type { #(#field_conversions),* }),
                extra_items,
            }
        }
    })
}

fn generate_transform_for_enum(
    input_enum: &DataEnum,
    pattern_type: impl ToTokens,
    construct_type: impl ToTokens,
    source_var: impl ToTokens,
    transform_field: &mut impl FnMut(&Field, &Ident, TokenStream) -> syn::Result<FieldTransform>,
) -> syn::Result<TransformResult> {
    let mut variant_conversions = Vec::new();
    let mut extra_items = Vec::new();
    for variant in input_enum.variants.iter() {
        let variant_ident = &variant.ident;
        let (variant_conversion, mut variant_extra_items) = match &variant.fields {
            Fields::Unit => (
                quote!(#pattern_type::#variant_ident => #construct_type::#variant_ident),
                Vec::new(),
            ),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let FieldResults {
                    bound_idents,
                    field_conversions,
                    extra_items,
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
                    .map(|(field, ident)| transform_field(field, &ident, ident.to_token_stream()))
                    .collect::<syn::Result<FieldResults>>()?;
                (
                    quote! {
                        #pattern_type::#variant_ident( #(#bound_idents),* ) => #construct_type::#variant_ident( #(#field_conversions),* )
                    },
                    extra_items,
                )
            }
            Fields::Named(FieldsNamed { named, .. }) => {
                let FieldResults {
                    bound_idents,
                    field_conversions,
                    extra_items,
                } = named
                    .iter()
                    .map(|field| {
                        let field_ident = field.ident.as_ref().unwrap();
                        transform_field(field, field_ident, field_ident.to_token_stream())
                    })
                    .collect::<syn::Result<FieldResults>>()?;
                (
                    quote! {
                        #pattern_type::#variant_ident { #(#bound_idents),* } => #construct_type::#variant_ident { #(#field_conversions),* }
                    },
                    extra_items,
                )
            }
        };
        variant_conversions.push(variant_conversion);
        extra_items.append(&mut variant_extra_items);
    }

    let variant_conversions = variant_conversions.into_iter();
    Ok(TransformResult {
        transformation: quote! {
            match #source_var {
                #(#variant_conversions),*
            }
        },
        extra_items,
    })
}

struct FieldResults {
    bound_idents: Vec<TokenStream>,
    field_conversions: Vec<TokenStream>,
    extra_items: Vec<TokenStream>,
}

impl FromIterator<FieldTransform> for FieldResults {
    fn from_iter<T: IntoIterator<Item = FieldTransform>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut bound_idents = Vec::new();
        let mut field_conversions = Vec::with_capacity(iter.size_hint().0);
        let mut extra_items = Vec::new();

        for FieldTransform {
            bound_ident,
            conversion_expr: conversion,
            extra_items: mut field_extra_items,
        } in iter
        {
            field_conversions.push(conversion);
            extra_items.append(&mut field_extra_items);
            if let Some(bound_ident) = bound_ident {
                bound_idents.push(bound_ident);
            }
        }

        Self {
            bound_idents,
            field_conversions,
            extra_items,
        }
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
