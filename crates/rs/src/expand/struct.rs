use cainome_parser::tokens::{Composite, Token};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::expand::types::CairoToRust;
use crate::expand::utils;

pub struct CairoStruct;

impl CairoStruct {
    pub fn expand_decl(composite: &Composite, add_typeshare: bool) -> TokenStream2 {
        if composite.is_builtin() {
            return quote!();
        }

        let struct_name = utils::str_to_ident(&composite.type_name_or_alias());

        let mut members: Vec<TokenStream2> = vec![];
        for inner in &composite.inners {
            let name = utils::str_to_ident(&inner.name);
            let ty = utils::str_to_type(&inner.token.to_rust_type());

            // r#{name} is not a valid identifier, thus we can't create an ident.
            // And with proc macro 2, we cannot do `quote!(r##name)`.
            // TODO: this needs to be done more elegantly...
            if &inner.name == "type" {
                members.push(quote!(r#type: #ty));
            } else if &inner.name == "move" {
                members.push(quote!(r#move: #ty));
            } else if &inner.name == "final" {
                members.push(quote!(r#final: #ty));
            } else {
                members.push(quote!(#name: #ty));
            }
        }

        let decl = if composite.is_generic() {
            let gen_args: Vec<Ident> = composite
                .generic_args
                .iter()
                .map(|(g, _)| utils::str_to_ident(g))
                .collect();

            // TODO: we may need Phantom fields here, in the case that
            // several generic are present in the struct definition,
            // but they are not all used.
            // Add one phantom for each generic type.
            // Those phantom fields are ignored by serde.

            // TODO: add a way for the user to specify which trait must be derived for the
            // generated structs. For now Serde is used to ensure easy serialization.

            quote! {
                #[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
                pub struct #struct_name<#(#gen_args),*> {
                    #(pub #members),*
                }
            }
        } else {
            quote! {
                #[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
                pub struct #struct_name {
                    #(pub #members),*
                }
            }
        };

        if add_typeshare {
            quote! {
                #[typeshare]
                #decl
            }
        } else {
            decl
        }
    }

    pub fn expand_impl(composite: &Composite) -> TokenStream2 {
        if composite.is_builtin() {
            return quote!();
        }

        let struct_name = utils::str_to_ident(&composite.type_name_or_alias());

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

        let (impl_line, rust_type) = if composite.is_generic() {
            let gen_args: Vec<Ident> = composite
                .generic_args
                .iter()
                .map(|(g, _)| utils::str_to_ident(g))
                .collect();

            (
                utils::impl_with_gen_args(&struct_name, &gen_args),
                utils::rust_associated_type_gen_args(&struct_name, &gen_args),
            )
        } else {
            (
                quote!(impl #ccs::CairoSerde for #struct_name),
                quote!(
                    type RustType = Self;
                ),
            )
        };

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

                fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::FieldElement> {
                    let mut __out: Vec<starknet::core::types::FieldElement> = vec![];
                    #(#sers)*
                    __out
                }

                fn cairo_deserialize(__felts: &[starknet::core::types::FieldElement], __offset: usize) -> #ccs::Result<Self::RustType> {
                    let mut __offset = __offset;
                    #(#desers)*
                    Ok(#struct_name {
                        #(#names),*
                    })
                }
            }
        }
    }
}
