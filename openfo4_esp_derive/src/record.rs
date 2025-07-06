use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::Parse;
use syn::{Data, DeriveInput, LitStr, Type};

use crate::Error;

pub fn derive_record_impl(input: DeriveInput) -> Result<TokenStream2, Error> {
    let Data::Struct(data) = input.data else {
        return Err("Record macro can only be used on structs".into());
    };

    let mut fields = Vec::new();
    for field in data.fields {
        let mut key = None;
        let mut count = None;

        for attr in field.attrs {
            match attr
                .meta
                .path()
                .get_ident()
                .map(|i| i.to_string())
                .as_deref()
            {
                Some("record") => {
                    assert!(key.is_none());
                    let attr = attr.parse_args::<RecordAttr>()?;

                    key = Some(attr.key);
                }
                Some("count") => {
                    assert!(count.is_none());
                    let attr = attr.parse_args::<CountAttr>()?;

                    count = Some(attr.key);
                }
                _ => (),
            }
        }

        let Some(key) = key else {
            return Err("#[record] must be present on all fields".into());
        };

        fields.push(Field {
            ident: field
                .ident
                .clone()
                .unwrap_or_else(|| todo!("todo: support unnamed fields")),
            ty: field.ty,
            key,
            count,
        });
    }

    let struct_ident = input.ident;
    Ok(quote! {
        #[automatically_derived]
        impl crate::fields::Read for #struct_ident {
            type Error = crate::Error;

            fn read(_fields: &mut crate::fields::FieldMap, _: crate::Text<4>) -> ::core::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#fields)*
                })
            }
        }
    })
}

#[derive(Clone, Debug)]
struct Field {
    ident: Ident,
    ty: Type,
    key: String,
    count: Option<String>,
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let ty = &self.ty;

        let literals: TokenStream2 = self
            .key
            .as_bytes()
            .iter()
            .map(|&byte| {
                quote! {
                    #byte,
                }
            })
            .collect();

        let text = quote! { crate::Text::<4>([#literals]) };

        let value = quote! {
            match <#ty as crate::fields::Read>::read(_fields, #text) {
                Ok(value) => value,
                Err(err) => {
                    return Err(crate::Error::Record(Box::new(crate::RecordError {
                        field: #text,
                        error: err,
                    })));
                }
            }
        };

        tokens.extend(quote! {
            #ident: #value,
        });
    }
}

#[derive(Clone, Debug)]
struct RecordAttr {
    key: String,
}

impl Parse for RecordAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key = input.parse::<LitStr>()?;
        Ok(Self { key: key.value() })
    }
}

#[derive(Clone, Debug)]
struct CountAttr {
    key: String,
}

impl Parse for CountAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key = input.parse::<LitStr>()?;
        Ok(Self { key: key.value() })
    }
}
