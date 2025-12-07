use cainome_parser::tokens::utils::is_builtin;
use cainome_parser::tokens::{CompositeLegacy, CoreBasic, Enum, Token};
use proc_macro2::TokenStream;
use quote::quote;

use crate::expand::types::CairoToRust;
use crate::expand::{utils, Expandable, ExpansionContext};

pub struct CairoEnum;

impl CairoEnum {
    pub fn expand_decl(composite: &CompositeLegacy, derives: &[String]) -> TokenStream {
        if is_builtin(&composite.type_path) {
            return quote!();
        }

        let enum_name = utils::str_to_ident(&composite.type_name_or_alias());

        let mut variants: Vec<TokenStream2> = vec![];

        for inner in &composite.inners {
            let name = utils::str_to_ident(&inner.name);
            let ty = utils::str_to_type(&inner.token.to_rust_type());

            let serde = utils::serde_hex_derive(&inner.token.to_rust_type());

            if inner.token.type_name() == "()" {
                variants.push(quote!(#serde #name));
            } else {
                variants.push(quote!(#serde #name(#ty)));
            }
        }

        let mut internal_derives = vec![];

        for d in derives {
            internal_derives.push(utils::str_to_type(d));
        }

        quote! {
            #[derive(#(#internal_derives,)*)]
            pub enum #enum_name {
                #(#variants),*
            }
        }
    }

    pub fn expand_impl(composite: &CompositeLegacy) -> TokenStream {
        if is_builtin(&composite.type_path) {
            return quote!();
        }

        let name_str = &composite.type_name_or_alias();
        let enum_name = utils::str_to_ident(name_str);

        let mut serialized_sizes: Vec<TokenStream2> = vec![];
        let mut serializations: Vec<TokenStream2> = vec![];
        let mut deserializations: Vec<TokenStream2> = vec![];

        for inner in &composite.inners {
            let variant_name = utils::str_to_ident(&inner.name);
            let ty = utils::str_to_type(&inner.token.to_rust_type_path());
            let variant_index = inner.index;

            // Tuples type used as rust type path must be surrounded
            // by angle brackets.
            let ty_punctuated = match inner.token.as_ref() {
                Token::Tuple(_) => quote!(<#ty>),
                _ => quote!(#ty),
            };

            if inner.token.type_name() == "()" {
                serializations.push(quote! {
                    #enum_name::#variant_name => usize::cairo_serialize(&#variant_index)
                });
                deserializations.push(quote! {
                    #variant_index => Ok(#enum_name::#variant_name)
                });
                serialized_sizes.push(quote! {
                    #enum_name::#variant_name => 1
                });
            } else {
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

        let ccs = utils::cainome_cairo_serde();

        serialized_sizes.push(quote! {
            _ => 0
        });

        serializations.push(quote! {
            _ => vec![]
        });

        deserializations.push(quote! {
            _ => return Err(#ccs::Error::Deserialize(format!("Index not handle for enum {}", #name_str)))
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
}

fn expand_enum_decl(e: &Enum, ctx: ExpansionContext) -> TokenStream {
    let type_name = e.type_path.split("::").last().unwrap();

    let enum_name = utils::str_to_ident(&type_name);

    let mut variants: Vec<TokenStream> = vec![];

    for inner in &e.variants {
        let name = utils::str_to_ident(&inner.name);

        let token = &*inner.token.borrow();
        let serde = utils::serde_hex_derive(&token.to_rust_type());

        match &*inner.token.borrow() {
            Token::Basic(CoreBasic { type_path }) if type_path == "()" => {
                variants.push(quote!(#serde #name));
            }
            _ => {
                let ty = utils::str_to_type(&token.to_rust_type());
                variants.push(quote!(#serde #name(#ty)));
            }
        }
    }

    let mut internal_derives = vec![];

    for d in ctx.derives {
        internal_derives.push(utils::str_to_type(&d));
    }

    quote! {
        #[derive(#(#internal_derives,)*)]
        pub enum #enum_name {
            #(#variants),*
        }
    }
}

fn expand_enum_impl(e: &Enum, ctx: ExpansionContext) {
    let type_name = e.type_path.split("::").last().unwrap();
    let enum_name = utils::str_to_ident(type_name);

    let mut serialized_sizes: Vec<TokenStream> = vec![];
    let mut serializations: Vec<TokenStream> = vec![];
    let mut deserializations: Vec<TokenStream> = vec![];

    for inner in &e.variants {
        let variant_name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();

        let ty = utils::str_to_type(&token.to_rust_type_path());
        let variant_index = inner.index;

        // Tuples type used as rust type path must be surrounded
        // by angle brackets.
        let ty_punctuated = match inner.token.as_ref() {
            Token::Tuple(_) => quote!(<#ty>),
            _ => quote!(#ty),
        };

        if inner.token.type_name() == "()" {
            serializations.push(quote! {
                #enum_name::#variant_name => usize::cairo_serialize(&#variant_index)
            });
            deserializations.push(quote! {
                #variant_index => Ok(#enum_name::#variant_name)
            });
            serialized_sizes.push(quote! {
                #enum_name::#variant_name => 1
            });
        } else {
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

    let ccs = utils::cainome_cairo_serde();

    serialized_sizes.push(quote! {
        _ => 0
    });

    serializations.push(quote! {
        _ => vec![]
    });

    deserializations.push(quote! {
            _ => return Err(#ccs::Error::Deserialize(format!("Index not handle for enum {}", #name_str)))
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
    fn expand(&self, expansion_context: ExpansionContext) -> Vec<TokenStream> {
        return vec![expand_enum_decl(self, expansion_context)];
    }
}
