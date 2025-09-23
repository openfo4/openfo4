use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Data, DeriveInput, LitStr, Type};

use crate::Error;

pub fn derive_record_impl(input: DeriveInput) -> Result<TokenStream2, Error> {
    let attrs = StructAttributes::parse(&input.attrs);

    let Data::Struct(data) = input.data else {
        return Err("Record macro can only be used on structs".into());
    };

    let mut fields = Vec::new();
    for field in data.fields {
        let mut key = None;
        let mut count = None;
        let mut unordered = false;

        for attr in field.attrs {
            match attr
                .meta
                .path()
                .get_ident()
                .map(|i| i.to_string())
                .as_deref()
            {
                Some("record") => match attr.parse_args::<RecordAttr>()? {
                    RecordAttr::Key(k) => {
                        assert!(key.is_none());
                        key = Some(k);
                    }
                    RecordAttr::Unordered => {
                        unordered = true;
                    }
                },
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
            read_unordered: unordered,
        });
    }

    let mut read_impl: TokenStream2 = fields.iter().map(|f| f.expand_read_impl()).collect();
    let read_next_impl: TokenStream2 = fields.iter().map(|f| f.expand_read_next_impl()).collect();

    if attrs.is_ordered {
        read_impl = read_next_impl.clone();
    }

    let struct_ident = input.ident;
    Ok(quote! {
        #[automatically_derived]
        impl crate::fields::Read for #struct_ident {
            type Error = crate::Error;

            fn read(_fields: &mut crate::fields::FieldMap, _: crate::Text<4>) -> ::core::result::Result<Self, Self::Error> {
                Ok(Self {
                    #read_impl
                })
            }

            fn read_next(_fields: &mut crate::fields::FieldMap, _: crate::Text<4>) -> ::core::result::Result<Self, Self::Error> {
                Ok(Self {
                    #read_next_impl
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
    read_unordered: bool,
}

impl Field {
    fn expand_read_impl(&self) -> TokenStream2 {
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

        quote! {
            #ident: #value,
        }
    }

    fn expand_read_next_impl(&self) -> TokenStream2 {
        if self.read_unordered {
            return self.expand_read_impl();
        }

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
            match <#ty as crate::fields::Read>::read_next(_fields, #text) {
                Ok(value) => value,
                Err(err) => {
                    return Err(crate::Error::Record(Box::new(crate::RecordError {
                        field: #text,
                        error: err,
                    })));
                }
            }
        };

        quote! {
            #ident: #value,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RecordAttr {
    Key(String),
    Unordered,
}

impl Parse for RecordAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        match input.parse::<Ident>() {
            Ok(ident) => match &*ident.to_string() {
                "unordered" => Ok(Self::Unordered),
                _ => panic!("unknown field attribute"),
            },
            Err(_) => {
                let key = input.parse::<LitStr>()?;
                Ok(Self::Key(key.value()))
            }
        }
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

#[derive(Clone, Debug)]
struct StructAttributes {
    pub is_ordered: bool,
}

impl StructAttributes {
    fn parse(attrs: &[Attribute]) -> Self {
        let mut is_ordered = false;

        for attr in attrs {
            let Some(name) = attr.path().get_ident() else {
                continue;
            };

            if name.to_string() != "record" {
                continue;
            }

            match attr.parse_args::<StructAttribute>().unwrap() {
                StructAttribute::Ordered => is_ordered = true,
            }
        }

        Self { is_ordered }
    }
}

#[derive(Clone, Debug)]
enum StructAttribute {
    Ordered,
}

impl Parse for StructAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match &*input.parse::<Ident>()?.to_string() {
            "ordered" => Ok(Self::Ordered),
            _ => todo!(),
        }
    }
}
