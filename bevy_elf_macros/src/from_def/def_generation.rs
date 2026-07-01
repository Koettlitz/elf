use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Field, Fields, Generics, Visibility, parse2,
    punctuated::Punctuated, token::Comma,
};

use crate::{CratePath, ELF_MODULE_PATH, from_def::FieldAttr};

pub fn generate_def_for(
    derive_input: &DeriveInput,
    def_type: &syn::Type,
) -> Result<TokenStream, syn::Error> {
    let def_type_definition = match &derive_input.data {
        Data::Struct(input_struct) => generate_def_for_struct(
            input_struct,
            &derive_input.vis,
            &def_type,
            &derive_input.generics,
        ),
        Data::Enum(input_enum) => generate_def_for_enum(
            &input_enum,
            &derive_input.vis,
            &def_type,
            &derive_input.generics,
        ),
        Data::Union(_) => Err(syn::Error::new_spanned(
            derive_input,
            "unions are not supported",
        )),
    }?;
    Ok(quote! {
        #[derive(serde::Serialize, serde::Deserialize)]
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
    let from_def_trait = match FieldAttr::parse(&field.attrs)? {
        Some(attr) if attr.omit_def_field() => {
            return Ok(None);
        }
        Some(FieldAttr::FromDef { spec: Some(_), .. })
        | Some(FieldAttr::FromDef {
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
    field.attrs.clear();
    field.ty = parse2(quote!(<#field_type as #from_def_trait>::Def))?;
    Ok(Some(field))
}

#[cfg(test)]
mod test {
    use super::generate_def_for;
    use quote::quote;
    use syn::{DeriveInput, ItemStruct, parse2};

    #[test]
    fn test_def_generation() {
        let input_struct = quote! {
            struct TestAsset<T: ops::Add> {
                name: String,
                fancy: Vec<Rc<RefCell<T::Output>>>,
                handle: Handle<HurensohnAsset<'_>>,
                nested: Vec<Rc<RefCell<Handle<T::Output>>>>,
            }
        }
        .into();
        let derive_input: DeriveInput = parse2(input_struct).unwrap();
        let def_type = syn::parse_str("TestDef").unwrap();
        let generated = generate_def_for(&derive_input, &def_type).unwrap();
        let expected = quote! {
            #[derive(serde::Serialize, serde::Deserialize)]
            struct TestDef<T: ops::Add> {
                name: <String as bevy_elf::FromDef>::Def,
                fancy: <Vec<Rc<RefCell<T::Output>>> as bevy_elf::FromDef>::Def,
                handle: <Handle<HurensohnAsset<'_>> as bevy_elf::FromDef>::Def,
                nested: <Vec<Rc<RefCell<Handle<T::Output>>>> as bevy_elf::FromDef>::Def
            }
        };
        let generated: ItemStruct = parse2(generated).unwrap();
        let expected: ItemStruct = parse2(expected).unwrap();
        let generated = quote!(#generated).to_string();
        let expected = quote!(#expected).to_string();
        assert_eq!(generated, expected);
    }
}
