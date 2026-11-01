use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Expr, Fields, Lit, LitStr, Type, parse_macro_input};

#[proc_macro_derive(ConfigSection)]
pub fn config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let mut write_fields = Vec::new();

    match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    let mut comment = String::new();
                    for attr in &field.attrs {
                        if attr.meta.path().is_ident("doc") {
                            let attr = attr.meta.require_name_value().unwrap();
                            let Expr::Lit(lit) = &attr.value else { todo!() };
                            let Lit::Str(lit) = &lit.lit else { todo!() };

                            if !comment.is_empty() {
                                comment.push('\n');
                            }

                            comment.push_str(lit.value().trim());
                        }
                    }

                    let ident = field.ident.as_ref().unwrap();
                    let default = quote! { default.#ident };

                    write_fields.push(Field {
                        ty: field.ty.clone(),
                        ident: field.ident.clone().unwrap(),
                        comment,
                        default,
                    });
                }
            }
            _ => todo!(),
        },
        _ => todo!(),
    }

    let ident = &input.ident;

    quote! {
        #[automatically_derived]
        impl openfo4_config::ConfigSection for #ident {
            fn write(table: &mut openfo4_config::toml_edit::Table) -> bool {
                let mut updated = false;

                let default = <Self as core::default::Default>::default();

                #(
                    updated |= #write_fields;
                )*

                updated
            }
        }
    }
    .into()
}

struct Field {
    ty: Type,
    ident: Ident,
    comment: String,
    default: TokenStream2,
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let key = self.ident.to_string();
        let mut prefix = self.comment.replace("\n", "\n# ");
        prefix.insert_str(0, "# ");
        prefix.push_str("\n#\n# Default Value: ");

        let prefix = LitStr::new(&prefix, Span::call_site());
        let default = &self.default;
        let ty = &self.ty;

        tokens.extend(quote! {
            <#ty as openfo4_config::ConfigValue>::insert(&#default, table, #key, #prefix)
        });
    }
}
