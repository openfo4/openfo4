mod record;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr, Generics, Ident,
    Lit, LitStr, Meta, Result, Type,
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Parse(#[from] syn::parse::Error),
    #[error("{0}")]
    Message(String),
}

impl<'a> From<&'a str> for Error {
    fn from(value: &'a str) -> Self {
        Self::Message(value.to_owned())
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

#[proc_macro_derive(Record, attributes(record))]
pub fn derive_record(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match record::derive_record_impl(input) {
        Ok(tt) => tt.into(),
        Err(err) => {
            let msg = LitStr::new(&err.to_string(), Span::call_site());
            quote! {
                compile_error!(#msg);
            }
            .into()
        }
    }
}

#[derive(Clone, Debug)]
struct Field {
    ident: Ident,
    ty: Type,
    info: FieldInfo,
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ident = &self.ident;
        let ty = &self.ty;

        let literals: TokenStream2 = self
            .info
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
            match <#ty as crate::ReadFromField>::read(&_fields, #text) {
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
struct FieldInfo {
    key: String,
}

impl Parse for FieldInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = input.parse::<LitStr>()?;
        Ok(Self { key: key.value() })
    }
}

#[proc_macro_derive(BinaryStruct, attributes(primitive, default_value))]
pub fn derive_binary_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Struct(data) => derive_binary_struct_struct(&input.ident, &input.generics, data),
        Data::Enum(data) => derive_binary_struct_enum(&input.attrs, &input.ident, data),
        _ => quote! {
            compile_error!("The Record macro can only be used on structs");
        }
        .into(),
    }
}

fn derive_binary_struct_struct(
    struct_ident: &Ident,
    generics: &Generics,
    data: DataStruct,
) -> TokenStream {
    let generic_idents: TokenStream2 = generics
        .type_params()
        .map(|param| {
            let id = &param.ident;
            quote! { #id }
        })
        .collect();
    let where_bounds: TokenStream2 = generics
        .type_params()
        .map(|param| {
            let id = &param.ident;
            quote! {
                #id: crate::Decode,
                crate::Error: From<<#id as crate::Decode>::Error>,
            }
        })
        .collect();

    let fields: TokenStream2 = data
        .fields
        .iter()
        .map(|field| {
            let default_value = field.attrs.iter().find_map(|attr| {
                if attr.meta.path().get_ident().unwrap() == "default_value" {
                    match attr.meta {
                        Meta::Path(_) => Some(DefaultAttr::Default),
                        _ => Some(attr.parse_args::<DefaultAttr>().unwrap()),
                    }
                } else {
                    None
                }
            });

            let ty = &field.ty;

            let decode_field_expanded = match default_value {
                Some(default_value) => quote! {
                    if buf.has_remaining() {
                        <#ty as crate::Decode>::decode(&mut buf)?
                    } else {
                        #default_value
                    }
                },
                None => quote! {
                    <#ty as crate::Decode>::decode(&mut buf)?
                },
            };

            match &field.ident {
                Some(ident) => quote! { #ident: #decode_field_expanded, },
                None => quote! { #decode_field_expanded, },
            }
        })
        .collect();

    quote! {
        #[automatically_derived]
        impl<#generic_idents> crate::Decode for #struct_ident<#generic_idents>
        where
            #where_bounds
        {
            type Error = crate::Error;

            fn decode<__B>(mut buf: __B) -> ::core::result::Result<Self, Self::Error>
            where
                __B: bytes::Buf,
            {
                Ok(Self {
                    #fields
                })
            }
        }
    }
    .into()
}

fn derive_binary_struct_enum(
    attrs: &[Attribute],
    enum_ident: &Ident,
    data: DataEnum,
) -> TokenStream {
    let size = attrs
        .iter()
        .find(|a| {
            a.meta
                .path()
                .is_ident(&Ident::new("primitive", Span::call_site()))
        })
        .unwrap();

    let ty = size.parse_args::<Type>().unwrap();

    let mut variants = Vec::new();
    for variant in data.variants.iter() {
        if !variant.fields.is_empty() {
            return quote! { compile_error!("Enum cannot have fields") }.into();
        }

        let Some((_, discriminant)) = &variant.discriminant else {
            return quote! { compile_error!("Every variant must have a distinct disciminant") }
                .into();
        };

        let Expr::Lit(lit) = discriminant else {
            todo!()
        };
        let Lit::Int(lit) = &lit.lit else { todo!() };

        variants.push((&variant.ident, lit));
    }

    let match_branches: TokenStream2 = variants
        .iter()
        .map(|(ident, lit)| {
            quote! {
                Ok(#lit) => Ok(Self::#ident),
            }
        })
        .collect();

    quote! {
        impl crate::Decode for #enum_ident {
            type Error = crate::Error;

            fn decode<__B>(mut buf: __B) -> ::core::result::Result<Self, Self::Error>
            where
                __B: bytes::Buf,
            {
                match <#ty as crate::Decode>::decode(buf) {
                    #match_branches
                    Ok(v) => Err(crate::Error::BadEnumValue(stringify!(#enum_ident), v.into())),
                    Err(err) => Err(err),
                }
            }

        }
    }
    .into()
}

#[derive(Clone, Debug)]
struct BinaryField {
    ident: Ident,
    ty: Type,
}

#[derive(Clone, Debug)]
enum DefaultAttr {
    Default,
    Expr(Expr),
}

impl Parse for DefaultAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            Ok(Self::Default)
        } else {
            input.parse::<Expr>().map(Self::Expr)
        }
    }
}

impl ToTokens for DefaultAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Default => tokens.extend(quote! {
                ::core::default::Default::default()
            }),
            Self::Expr(expr) => tokens.extend(quote! {
                #expr
            }),
        }
    }
}
