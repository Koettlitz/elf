use proc_macro2::TokenStream;
use quote::quote;
use syn::parse2;

use crate::from_def::from_def_impl;

#[test]
fn simple_struct_generates_correctly() {
    let input = quote! {
        struct Foo {
            a: usize,
            b: Handle<Image>,
        }
    };

    let expected = quote! {
        #[derive(serde :: Serialize, serde :: Deserialize)]
        struct FooDef {
            a: <usize as bevy_elf::FromDef>::Def,
            b: <Handle<Image> as bevy_elf::FromDef>::Def,
        }

        impl bevy_elf::FromDef for Foo {
            type Def = FooDef;

            fn from_def(def: Self::Def, ctx: &mut bevy_asset::LoadContext<'_>) -> std::result::Result<Self, bevy_elf::FromDefError> {
                Ok(Self {
                    a: <usize as bevy_elf::FromDef>::from_def(def.a, ctx)?,
                    b: <Handle<Image> as bevy_elf::FromDef>::from_def(def.b, ctx)?,
                })
            }
        }
    };

    test_from_def(input, expected);
}

fn test_from_def(input: TokenStream, expected: TokenStream) {
    let output = from_def_impl(parse2(input).unwrap()).unwrap();
    let output = syn::parse_file(&output.to_string()).unwrap();
    let output = prettyplease::unparse(&output);
    let expected = syn::parse_file(&expected.to_string()).unwrap();
    let expected = prettyplease::unparse(&expected);
    assert_eq!(output, expected);
}
