use cainome_parser::tokens::{CoreBasic, Enum, NamedToken, Token};
use proc_macro2::TokenStream;
use quote::quote;

use crate::expand::types::CairoToRust;
use crate::expand::{utils, Expandable, ExpansionContext, Module};

pub fn enum_declaration(
    type_name: &str,
    variants: &Vec<NamedToken>,
    ctx: &ExpansionContext,
) -> TokenStream {
    let enum_name = utils::str_to_ident(&type_name);

    let mut generated_variants: Vec<TokenStream> = vec![];

    for inner in variants {
        let name = utils::str_to_ident(&inner.name);

        let token = &*inner.token.borrow();
        let serde = utils::serde_hex_derive(&token.to_rust_type());

        match &*inner.token.borrow() {
            Token::Basic(CoreBasic { type_path }) if type_path == "()" => {
                generated_variants.push(quote!(#serde #name));
            }
            _ => {
                let ty = utils::str_to_type(&token.to_rust_type());
                generated_variants.push(quote!(#serde #name(#ty)));
            }
        }
    }

    let mut internal_derives = vec![];

    for d in ctx.derives.iter() {
        internal_derives.push(utils::str_to_type(d));
    }

    let derive = if internal_derives.len() > 0 {
        quote! { #[derive(#(#internal_derives,)*)] }
    } else {
        quote! {}
    };

    quote! {
        #derive

        pub enum #enum_name {
            #(#generated_variants),*
        }
    }
}

pub fn enum_implementation(
    type_name: &str,
    variants: &Vec<NamedToken>,
    _ctx: &ExpansionContext,
) -> TokenStream {
    let enum_name = utils::str_to_ident(type_name);
    let enum_name_str = utils::str_to_litstr(type_name);

    let mut serialized_sizes: Vec<TokenStream> = vec![];
    let mut serializations: Vec<TokenStream> = vec![];
    let mut deserializations: Vec<TokenStream> = vec![];

    for (variant_index, inner) in variants.iter().enumerate() {
        let variant_name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();

        let ty = utils::str_to_type(&token.to_rust_type_path());

        // Tuples type used as rust type path must be surrounded
        // by angle brackets.
        let ty_punctuated = match &*inner.token.borrow() {
            Token::Tuple(_) => quote!(<#ty>),
            _ => quote!(#ty),
        };

        match &*inner.token.borrow() {
            Token::Basic(CoreBasic { type_path }) if type_path == "()" => {
                serializations.push(quote! {
                    #enum_name::#variant_name => usize::cairo_serialize(&#variant_index)
                });
                deserializations.push(quote! {
                    #variant_index => Ok(#enum_name::#variant_name)
                });
                serialized_sizes.push(quote! {
                    #enum_name::#variant_name => 1
                });
            }
            _ => {
                serializations.push(quote! {
                    #enum_name::#variant_name(val) => {
                        let mut temp = vec![];
                        temp.extend(usize::cairo_serialize(&#variant_index));
                        temp.extend(#ty_punctuated::cairo_serialize(val));
                        temp
                    }
                });
                deserializations.push(quote! {
                    #variant_index => Ok(#enum_name::#variant_name(#ty_punctuated::cairo_deserialize(__felts, __offset + 1)?))
                });
                // +1 because we have to handle the variant index also.
                serialized_sizes.push(quote! {
                    #enum_name::#variant_name(val) => #ty_punctuated::cairo_serialized_size(val) + 1
                })
            }
        }
    }

    let ccs = utils::cainome_cairo_serde();

    serialized_sizes.push(quote! {
        _ => 0
    });

    serializations.push(quote! {
        _ => vec![]
    });

    deserializations.push(quote! {
        _ => return Err(#ccs::Error::Deserialize(format!("Index not handle for enum {}", #enum_name_str)))
    });

    let (impl_line, rust_type) = (
        quote!(impl #ccs::CairoSerde for #enum_name),
        quote!(
            type RustType = Self;
        ),
    );

    quote! {
        #impl_line {

            #rust_type

            const SERIALIZED_SIZE: std::option::Option<usize> = std::option::Option::None;

            #[inline]
            fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
                match __rust {
                    #(#serialized_sizes),*
                }
            }

            fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
                match __rust {
                    #(#serializations),*
                }
            }

            fn cairo_deserialize(__felts: &[starknet::core::types::Felt], __offset: usize) -> #ccs::Result<Self::RustType> {
                let __f = __felts[__offset];
                let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());

                match __index as usize {
                    #(#deserializations),*
                }

            }
        }
    }
}

impl Expandable for Enum {
    fn expand(&self, ctx: &ExpansionContext) -> Vec<super::Module> {
        let module = self.type_module();
        let name = self.type_name();

        let declaration = enum_declaration(&name, &self.variants, ctx);
        let implementation = enum_implementation(&name, &self.variants, ctx);

        let item = quote! {
            #declaration

            #implementation
        };

        vec![Module::new(&module).add_item(&name, item)]
    }
}

#[cfg(test)]
mod tests {
    use cainome_parser::{
        tokens::{Enum, NamedToken},
        TypeRegistry,
    };
    use proc_macro2::TokenStream;
    use quote::ToTokens;
    use syn::{parse_quote, ItemEnum};

    use crate::expand::{render, Expandable, ExpansionContext};

    fn assert_code_has<T: ToTokens>(generated: &TokenStream, expected: &T, message: &str) {
        let file: syn::File = syn::parse2(generated.clone()).expect("expected file-like tokens");

        let expected_str = expected.to_token_stream().to_string();

        let has_match = file.items.iter().any(|item| {
            if let syn::Item::Enum(s) = item {
                let s = s.to_token_stream().to_string();
                s == expected_str
            } else {
                false
            }
        });

        assert!(
            has_match,
            "{}. Expected: {} In: {}",
            message,
            expected_str,
            generated.to_string()
        );
    }

    #[test]
    fn test_enum_expand_empty() {
        let registry = TypeRegistry::new();

        let enumeration = Enum::new("my::Enum".to_string(), &registry).unwrap();

        let ctx = ExpansionContext::new("ContractName");

        let generated = render(enumeration.expand(&ctx));

        let expected: ItemEnum = parse_quote! {
            pub enum Enum {}
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_enum_expand_simple_variants() {
        let registry = TypeRegistry::new();

        let mut enumeration = Enum::new("my::Enum".to_string(), &registry).unwrap();

        enumeration.variants.push(NamedToken {
            name: "variant1".to_string(),
            token: registry.get("felt").unwrap(),
        });

        let ctx = ExpansionContext::new("ContractName");

        let generated = render(enumeration.expand(&ctx));

        let expected: ItemEnum = parse_quote! {
            pub enum Enum {
                variant1(starknet::core::types::Felt)
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }
}
