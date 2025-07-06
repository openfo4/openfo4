use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    Data, DeriveInput, Expr, ExprLit, ItemStruct, Lit, LitInt, LitStr, Type, UnOp,
    parse_macro_input,
};

#[proc_macro_derive(HkStruct)]
pub fn derive_hk_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let data = match &input.data {
        Data::Struct(data) => data,
        _ => todo!(),
    };

    let struct_ident = &input.ident;
    let impl_body = data
        .fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;

            let decode = quote! {
                match <#ty as crate::Decode>::decode(reader) {
                    Ok(value) => value,
                    Err(mut err) => {
                        let frame = ::core::concat!(::core::stringify!(#struct_ident), "::", ::core::stringify!(#ident));
                        err.callstack.insert(0, frame);
                        return ::core::result::Result::Err(err);
                    }
                }
            };

            quote! {
                #ident: #decode,
            }
        })
        .collect::<TokenStream2>();

    quote! {
        #[automatically_derived]
        impl crate::Decode for #struct_ident {
            fn decode(reader: &mut crate::Reader<'_>) -> Result<Self, crate::Error> {
                Ok(Self {
                    #impl_body
                })
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn signature(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let args = parse_macro_input!(args as LitInt);

    let ident = &input.ident;
    let signature_impl = quote! {
        impl crate::Signature for #ident {
            const ID: u32 = #args;
        }
    };

    [
        TokenStream::from(quote! { #input }),
        TokenStream::from(signature_impl),
    ]
    .into_iter()
    .collect()
}

#[proc_macro_derive(HkEnum, attributes(size))]
pub fn derive_hk_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Some(size) = input.attrs.iter().find(|attr| {
        attr.meta
            .path()
            .get_ident()
            .is_some_and(|v| v.to_string() == "size")
    }) else {
        return emit_compile_error("#[size] attribute must be present in the enum");
    };

    let size = size.parse_args::<Type>().unwrap();

    let data = match &input.data {
        Data::Enum(data) => data,
        _ => return emit_compile_error("#[derive(HkEnum)] only works on enums"),
    };

    let enum_ident = &input.ident;

    let mut variants = TokenStream2::new();
    for variant in &data.variants {
        let Some((_, discriminant)) = &variant.discriminant else {
            panic!("explicit discriminant must be set");
        };

        let mut discriminant = discriminant;

        let mut unops = Vec::new();
        let mut value: i64;

        loop {
            match discriminant {
                Expr::Lit(lit) => match &lit.lit {
                    Lit::Int(v) => {
                        match v.base10_parse() {
                            Ok(v) => value = v,
                            Err(err) => {
                                return emit_compile_error(format!(
                                    "failed to parse discrimnant value: {}",
                                    err
                                ));
                            }
                        }

                        break;
                    }
                    _ => return emit_compile_error("expected integer literal in discriminant"),
                },
                Expr::Unary(expr) => {
                    unops.push(expr.op);
                    discriminant = &*expr.expr;
                }
                _ => return emit_compile_error(format!("expected literal in discriminant")),
            }
        }

        for op in unops.iter().rev() {
            match op {
                UnOp::Neg(_) => value = -value,
                UnOp::Not(_) => value = !value,
                _ => (),
            }
        }

        let lit = LitInt::new(&value.to_string(), Span::call_site());
        let variant_ident = &variant.ident;
        variants.extend(quote! {
            Ok(#lit) => ::core::result::Result::Ok(Self :: #variant_ident),
        });
    }

    quote! {
        #[automatically_derived]
        impl crate::Decode for #enum_ident {
            fn decode(reader: &mut crate::Reader<'_>) -> Result<Self, crate::Error> {
                let position_before_decode = reader.cursor.position();
                match <#size as crate::Decode>::decode(reader) {
                    #variants
                    Err(mut err) => {
                        let frame = ::core::stringify!(#enum_ident);
                        err.callstack.insert(0, frame);
                        ::core::result::Result::Err(err)
                    }
                    Ok(value) => {
                        let frame = ::core::stringify!(#enum_ident);
                        Err(crate::Error {
                            kind: crate::ErrorKind::UnknownEnumValue {
                                ty: ::core::stringify!(#enum_ident),
                                value: value.to_string(),
                            },
                            position: position_before_decode,
                            callstack: vec![frame],
                        })
                    }
                }
            }
        }
    }
    .into()
}

fn emit_compile_error<T>(msg: T) -> TokenStream
where
    T: ToString,
{
    let msg = msg.to_string();
    let lit = Lit::Str(LitStr::new(&msg, Span::call_site()));

    quote! {
        ::core::compile_error!(#lit);
    }
    .into()
}
