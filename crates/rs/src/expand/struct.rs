use cainome_parser::tokens::{Composite, Token};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::expand::types::CairoToRust;
use crate::expand::utils;

pub struct CairoStruct;

impl CairoStruct {
    pub fn expand_decl(composite: &Composite, derives: &[String]) -> TokenStream2 {
        if composite.is_builtin() {
            return quote!();
        }

        let struct_name = utils::str_to_ident(&composite.type_name_or_alias());

        let mut members: Vec<TokenStream2> = vec![];
        for inner in &composite.inners {
            let name = utils::str_to_ident(&inner.name);
            let ty = utils::str_to_type(&inner.token.to_rust_type());

            let serde = utils::serde_hex_derive(&inner.token.to_rust_type());

            // r#{name} is not a valid identifier, thus we can't create an ident.
            // And with proc macro 2, we cannot do `quote!(r##name)`.
            // TODO: this needs to be done more elegantly...
            if &inner.name == "type" {
                members.push(quote!(#serde pub r#type: #ty));
            } else if &inner.name == "move" {
                members.push(quote!(#serde pub r#move: #ty));
            } else if &inner.name == "final" {
                members.push(quote!(#serde pub r#final: #ty));
            } else {
                members.push(quote!(#serde pub #name: #ty));
            }
        }

        let mut internal_derives = vec![];

        for d in derives {
            internal_derives.push(utils::str_to_type(d));
        }

        quote! {
            #[derive(#(#internal_derives,)*)]
            pub struct #struct_name {
                #(#members),*
            }
        }
    }

    pub fn expand_impl(composite: &Composite) -> TokenStream2 {
        if composite.is_builtin() {
            return quote!();
        }

        let struct_name = utils::str_to_ident(&composite.type_name_or_alias());
        let struct_name_str = utils::str_to_litstr(&composite.type_name_or_alias());

        let mut sizes: Vec<TokenStream2> = vec![];
        let mut sers: Vec<TokenStream2> = vec![];
        let mut desers: Vec<TokenStream2> = vec![];
        let mut names: Vec<TokenStream2> = vec![];

        for inner in &composite.inners {
            let name = utils::str_to_ident(&inner.name);
            let ty = utils::str_to_type(&inner.token.to_rust_type_path());

            // Tuples type used as rust type path item path must be surrounded
            // by angle brackets.
            let ty_punctuated = match inner.token {
                Token::Tuple(_) => quote!(<#ty>),
                _ => quote!(#ty),
            };

            // r#{name} is not a valid identifier, thus we can't create an ident.
            // And with proc macro 2, we cannot do `quote!(r##name)`.
            // TODO: this needs to be done more elegantly...
            if &inner.name == "type" {
                names.push(quote!(r#type));

                sizes.push(quote! {
                    __size += #ty_punctuated::cairo_serialized_size(&__rust.r#type);
                });

                sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.r#type));));

                desers.push(quote! {
                    let r#type = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                    __offset += #ty_punctuated::cairo_serialized_size(&r#type);
                });
            } else if &inner.name == "move" {
                names.push(quote!(r#move));

                sizes.push(quote! {
                    __size += #ty_punctuated::cairo_serialized_size(&__rust.r#move);
                });

                sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.r#move));));

                desers.push(quote! {
                    let r#move = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                    __offset += #ty_punctuated::cairo_serialized_size(&r#move);
                });
            } else if &inner.name == "final" {
                names.push(quote!(r#final));

                sizes.push(quote! {
                    __size += #ty_punctuated::cairo_serialized_size(&__rust.r#final);
                });

                sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.r#final));));

                desers.push(quote! {
                    let r#final = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                    __offset += #ty_punctuated::cairo_serialized_size(&r#final);
                });
            } else {
                names.push(quote!(#name));

                sizes.push(quote! {
                    __size += #ty_punctuated::cairo_serialized_size(&__rust.#name);
                });

                sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.#name));));

                desers.push(quote! {
                    let #name = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                    __offset += #ty_punctuated::cairo_serialized_size(&#name);
                });
            }
        }

        let ccs = utils::cainome_cairo_serde();
        let snrs_types = utils::snrs_types();
        let snrs_utils = utils::snrs_utils();

        let event_impl = if composite.is_event {
            quote! {
                impl #struct_name {
                    pub fn event_selector() -> #snrs_types::Felt {
                        // Ok to unwrap since the event name comes from the ABI, which is already validated.
                        #snrs_utils::get_selector_from_name(#struct_name_str).unwrap()
                    }

                    pub fn event_name() -> &'static str {
                        #struct_name_str
                    }
                }
            }
        } else {
            quote!()
        };

        let (impl_line, rust_type) = (
            quote!(impl #ccs::CairoSerde for #struct_name),
            quote!(
                type RustType = Self;
            ),
        );

        quote! {
            #impl_line {

                #rust_type

                const SERIALIZED_SIZE: std::option::Option<usize> = None;

                #[inline]
                fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
                    let mut __size = 0;
                    #(#sizes)*
                    __size
                }

                fn cairo_serialize(__rust: &Self::RustType) -> Vec<#snrs_types::Felt> {
                    let mut __out: Vec<#snrs_types::Felt> = vec![];
                    #(#sers)*
                    __out
                }

                fn cairo_deserialize(__felts: &[#snrs_types::Felt], __offset: usize) -> #ccs::Result<Self::RustType> {
                    let mut __offset = __offset;
                    #(#desers)*
                    Ok(#struct_name {
                        #(#names),*
                    })
                }
            }

            #event_impl
        }
    }
}
