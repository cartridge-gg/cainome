use cainome_parser::tokens::{Composite, CompositeInnerKind, Token};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{LitStr, Type};

use crate::expand::types::CairoToRust;
use crate::expand::utils;

pub struct CairoEnumEvent;

/// Expansion of Cairo event enumeration.
impl CairoEnumEvent {
    pub fn expand(composite: &Composite, enums: &[Token], structs: &[Token]) -> TokenStream2 {
        if !composite.is_event {
            return quote!();
        }

        let depth = 0;
        let content = Self::expand_event_enum(composite, depth, enums, structs, None);

        let event_name = utils::str_to_ident(&composite.type_name_or_alias());

        let snrs_types = utils::snrs_types();
        let ccs = utils::cainome_cairo_serde();

        quote! {
            impl TryFrom<#snrs_types::EmittedEvent> for #event_name {
                type Error = String;

                fn try_from(event: #snrs_types::EmittedEvent) -> Result<Self, Self::Error> {
                    use #ccs::CairoSerde;

                    if event.keys.is_empty() {
                        return Err("Event has no key".to_string());
                    }

                    #content

                    Err(format!("Could not match any event from keys {:?}", event.keys))
                }
            }
        }
    }

    pub fn expand_event_enum(
        composite: &Composite,
        depth: usize,
        enums: &[Token],
        structs: &[Token],
        outter_enum: Option<Type>,
    ) -> TokenStream2 {
        let mut variants = vec![];

        let event_name_str = composite.type_name_or_alias();
        let event_name = utils::str_to_ident(&composite.type_name_or_alias());

        let snrs_utils = utils::snrs_utils();

        for variant in &composite.inners {
            let selector_key_offset = utils::str_to_litint(&depth.to_string());

            let variant_ident = utils::str_to_ident(&variant.name);
            let variant_name_str = utils::str_to_litstr(&variant.name);

            let variant_type_path = variant.token.type_path();
            let variant_type_name =
                utils::str_to_ident(&variant.token.to_composite().unwrap().type_name_or_alias());

            let (variant_is_enum, variant_token) = if let Some(t) =
                enums.iter().find(|t| t.type_path() == variant_type_path)
            {
                (true, t)
            } else if let Some(t) = structs.iter().find(|t| t.type_path() == variant_type_path) {
                (false, t)
            } else {
                panic!(
                    "The type {} was not found in existing enums and structs.",
                    variant_type_path
                );
            };

            let is_flat = variant.kind == CompositeInnerKind::Flat;

            // If it's flat, the compiler enforces the type to be an enum.
            #[allow(clippy::collapsible_else_if)]
            let content = if is_flat {
                // TODO: need recursion here...
                let outter = utils::str_to_type(&format!("{}::{}", event_name_str, &variant.name));
                Self::expand_event_enum(
                    variant_token.to_composite().unwrap(),
                    depth,
                    enums,
                    structs,
                    Some(outter),
                )
            } else {
                if variant_is_enum {
                    // Not flat, check the first key that must match the current variant name.
                    let outter =
                        utils::str_to_type(&format!("{}::{}", event_name_str, &variant.name));
                    let inner_content = Self::expand_event_enum(
                        variant_token.to_composite().unwrap(),
                        depth + 1,
                        enums,
                        structs,
                        Some(outter),
                    );

                    quote! {
                        let selector = event.keys[#selector_key_offset];
                        if selector == #snrs_utils::get_selector_from_name(#variant_name_str).unwrap_or_else(|_| panic!("Invalid selector for {}", #variant_name_str)) {
                            #inner_content
                        }
                    }
                } else {
                    let (names, desers) = Self::expand_event_struct(
                        variant_token.to_composite().unwrap(),
                        variant_name_str.clone(),
                    );

                    let end_return = if let Some(ref o) = outter_enum {
                        quote! {
                            return Ok(#o(#event_name::#variant_ident(#variant_type_name {
                                #(#names),*
                            })))
                        }
                    } else {
                        quote! {
                            return Ok(#event_name::#variant_ident(#variant_type_name {
                                #(#names),*
                            }))
                        }
                    };

                    quote! {
                        let selector = event.keys[#selector_key_offset];
                        if selector == #snrs_utils::get_selector_from_name(#variant_name_str).unwrap_or_else(|_| panic!("Invalid selector for {}", #variant_name_str)) {
                            let mut key_offset = #selector_key_offset + 1;
                            let mut data_offset = 0;

                            #(#desers)*

                            #end_return
                        };
                    }
                }
            };

            variants.push(content);
            // If nested + struct -> expand the struct, and selector is key 0.
            // If nested + enum -> first check if key 0 is the selector. Then expand enum.
            // If flat -> only expand the inner enum without checking first key.
        }

        quote! {
            #(#variants)*
        }
    }

    fn expand_event_struct(
        composite: &Composite,
        variant_name: LitStr,
    ) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
        let mut desers_tokens = vec![];
        let mut names_tokens = vec![];

        for inner in &composite.inners {
            let name = utils::str_to_ident(&inner.name);
            let name_str = utils::str_to_litstr(&inner.name);

            let ty = utils::str_to_type(&inner.token.to_rust_type_path());
            // Tuples type used as rust type path item path must be surrounded
            // by angle brackets.
            let ty_punctuated = match inner.token {
                Token::Tuple(_) => quote!(<#ty>),
                _ => quote!(#ty),
            };

            match inner.kind {
                CompositeInnerKind::Key => {
                    desers_tokens.push(quote! {
                        let #name = match #ty_punctuated::cairo_deserialize(&event.keys, key_offset) {
                            Ok(v) => v,
                            Err(e) => return Err(format!("Could not deserialize field {} for {}: {:?}", #name_str, #variant_name, e)),
                        };
                        key_offset += #ty_punctuated::cairo_serialized_size(&#name);
                    });
                }
                CompositeInnerKind::Data => {
                    desers_tokens.push(quote! {
                        let #name = match #ty_punctuated::cairo_deserialize(&event.data, data_offset) {
                            Ok(v) => v,
                            Err(e) => return Err(format!("Could not deserialize field {} for {}: {:?}", #name_str, #variant_name, e)),
                        };
                        data_offset += #ty_punctuated::cairo_serialized_size(&#name);
                    });
                }
                _ => {}
            };

            names_tokens.push(quote!(#name));
        }

        (names_tokens, desers_tokens)
    }
}
