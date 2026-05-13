use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitInt};

pub fn derive_condition_function(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Enum(en) => {
            let mut output = TokenStream2::new();

            for variant in &en.variants {
                let variant_id = &variant.ident;

                let tag = variant
                    .attrs
                    .iter()
                    .find(|attr| attr.path().get_ident().as_ref().unwrap().to_string() == "tag")
                    .unwrap();
                let tag = tag.parse_args::<LitInt>().unwrap();

                let imp = match &variant.fields {
                    Fields::Unit => {
                        quote! {
                            Self::#variant_id
                        }
                    }
                    Fields::Named(fields) => {
                        let fields_imp: TokenStream2 = fields
                            .named
                            .iter()
                            .enumerate()
                            .map(|(index, field)| {
                                let field_id = field.ident.as_ref().unwrap();
                                let field_ty = &field.ty;

                                let parameter =
                                    Ident::new(&format!("parameter{}", index), Span::call_site());
                                let index = LitInt::new(&index.to_string(), Span::call_site());

                                quote! {
                                    #field_id: <#field_ty as ConditionParameter>::parse(
                                        data.#parameter,
                                        cis.get_mut(#index).map(|v| v.take()).flatten()
                                    )?,
                                }
                            })
                            .collect();

                        quote! {
                            Self::#variant_id {
                                #fields_imp
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let fields_imp: TokenStream2 = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(index, field)| {
                                // let field_id = field.ident.as_ref().unwrap();
                                let field_ty = &field.ty;
                                let parameter =
                                    Ident::new(&format!("parameter{}", index), Span::call_site());
                                let index = LitInt::new(&index.to_string(), Span::call_site());

                                quote! {
                                    <#field_ty as ConditionParameter>::parse(
                                        data.#parameter,
                                        cis.get_mut(#index).map(|v| v.take()).flatten()
                                    )?,
                                }
                            })
                            .collect();

                        quote! {
                            Self::#variant_id( #fields_imp )
                        }
                    }
                };

                output.extend(quote! {
                    #tag => Ok(#imp),
                });
            }

            let enum_id = &input.ident;

            quote! {
                impl #enum_id {
                    fn parse(data: RawConditionData, mut cis: [Option<ZString>; 2]) -> Result<Self, Error> {
                        match data.function {
                            #output
                            val => Err(Error::BadEnumValue("ConditionFunction", val.into())),
                        }
                    }
                }
            }
            .into()
        }
        _ => unreachable!(),
    }
}
