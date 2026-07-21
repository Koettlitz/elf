use std::borrow::Cow;

use syn::{Ident, LitStr, Token, parse::Parse};

pub struct SpecArgs<'a> {
    pub base_path: Cow<'a, LitStr>,
    pub extension: Option<Cow<'a, LitStr>>,
}

impl<'a> Parse for SpecArgs<'a> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut base_path: Option<LitStr> = None;
        let mut extension: Option<LitStr> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "base_path" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    base_path = Some(lit);
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

        let base_path =
            base_path.ok_or_else(|| syn::Error::new(input.span(), "`base_path` is required"))?;

        Ok(SpecArgs {
            base_path: Cow::Owned(base_path),
            extension: extension.map(Cow::Owned),
        })
    }
}
