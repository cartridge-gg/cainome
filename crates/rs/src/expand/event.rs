use std::collections::HashMap;

use crate::expand::{
    enumeration::{enum_declaration, enum_implementation},
    structure::{struct_declaration, struct_implementation},
    types::{get_additional_derive_requirements, CairoToRust},
    utils, Expandable, ExpansionContext, ExpansionContextFactory, ExpansionResult,
};
use cainome_parser::tokens::{Event, EventKind, Token};
use proc_macro2::TokenStream;
use quote::quote;

fn from_event_conversion_from_enum(event: &Event, ctx: &ExpansionContext) -> TokenStream {
    let snrs_utils = utils::snrs_utils();
    let full_path = ctx.apply_alias(&event.type_path);
    let event_name_str = full_path.split("::").last().unwrap().to_owned();
    let event_name = utils::str_to_ident(&event_name_str);

    let ccs = utils::str_to_type(&ctx.cainome_serde_path);

    let mut variants: Vec<TokenStream> = vec![];

    for inner in event.flat.iter() {
        let variant_name_str = utils::str_to_litstr(&inner.name);
        let variant_ident = utils::str_to_ident(&inner.name);

        let inner_token = &*inner.token.borrow();

        let Token::Event(event) = inner_token else {
            unreachable!("Flat event variant is always an event");
        };

        let inner_type_name = inner_token.to_rust_type(ctx);

        let inner_type_name_id = utils::str_to_type(&inner_type_name);

        let value = match event.kind {
            EventKind::Struct => unreachable!("Flat events can only be Enum"),
            EventKind::Enum => quote! {
                if keys[0] == #snrs_utils::get_selector_from_name(#variant_name_str).unwrap_or_else(|_| panic!("Invalid selector for {}", #variant_name_str)) {
                    let res = #inner_type_name_id::try_from_event(from_address, keys.to_vec(), data)?;
                    return Ok(#event_name::#variant_ident( res ));
                };
            },
        };

        variants.push(value);
    }

    for inner in event.nested.iter() {
        let variant_name_str = utils::str_to_litstr(&inner.name);
        let variant_ident = utils::str_to_ident(&inner.name);

        let inner_token = &*inner.token.borrow();
        let inner_type_name = inner_token.to_rust_type(ctx);

        let inner_type_name_str = utils::str_to_litstr(&inner_type_name);
        let inner_type_name_id = utils::str_to_type(&inner_type_name);

        let Token::Event(Event { kind, .. }) = inner_token else {
            unreachable!("Nested event variant is always an event");
        };

        let value = match kind {
            EventKind::Struct => quote! {
                if keys[0] == #snrs_utils::get_selector_from_name(#variant_name_str).unwrap_or_else(|_| panic!("Invalid selector for {}", #variant_name_str)) {
                    let res = #inner_type_name_id::cairo_deserialize(&data, 0)
                        .map_err(|e| format!("Could not deserialize {} event data: {:?}", #inner_type_name_str, e))?;

                    return Ok(#event_name::#variant_ident( res ));
                };
            },
            EventKind::Enum => quote! {
                if keys[0] == #snrs_utils::get_selector_from_name(#variant_name_str).unwrap_or_else(|_| panic!("Invalid selector for {}", #variant_name_str)) {
                    let res = #inner_type_name_id::try_from_event(from_address, keys[1..].to_vec(), data)?;
                    return Ok(#event_name::#variant_ident( res ));
                };
            },
        };

        variants.push(value);
    }

    quote! {
        use #ccs::CairoSerde;

        #(#variants)*
    }
}

impl Expandable for Event {
    fn expand(&self, ctx: &ExpansionContext) -> Vec<ExpansionResult> {
        let full_path = ctx.apply_alias(&self.type_path);
        let event_name = full_path.split("::").last().unwrap().to_owned();
        let event_name_str = utils::str_to_ident(&event_name);
        let snrs_types = utils::snrs_types();
        let snrs_utils = utils::snrs_utils();

        // Generate type definition, struct or enum, depending on the event type.
        match self.kind {
            cainome_parser::tokens::EventKind::Enum => {
                let event_conversion_implementation = from_event_conversion_from_enum(self, ctx);

                let variants = [self.nested.clone(), self.flat.clone()].concat();

                let ctx = ExpansionContextFactory::from(ctx)
                    .with_derives(get_additional_derive_requirements(&variants, ctx))
                    .build();

                let declaration = enum_declaration(
                    &full_path,
                    &event_name,
                    &variants,
                    &vec![],
                    &HashMap::new(),
                    &ctx,
                );
                let implementation = enum_implementation(
                    &full_path,
                    &event_name,
                    &variants,
                    &vec![],
                    &HashMap::new(),
                    &ctx,
                );

                let definition = quote! {

                    #declaration

                    #implementation

                    impl #event_name_str {
                        pub fn event_selector() -> #snrs_types::Felt {
                            #snrs_utils::get_selector_from_name(#event_name).unwrap()
                        }

                        pub fn event_name() -> &'static str {
                            #event_name
                        }

                        pub(crate) fn try_from_event(from_address: starknet::core::types::Felt, keys: Vec<starknet::core::types::Felt>, data: Vec<starknet::core::types::Felt>) -> Result<Self, String> {
                            if keys.is_empty() {
                                return Err("Event has no key".to_string());
                            }

                            #event_conversion_implementation

                            Err(format!("Could not match any event from keys {:?}", keys))
                        }
                    }

                    impl TryFrom<#snrs_types::EmittedEvent> for #event_name_str {
                        type Error = String;

                        fn try_from(event: #snrs_types::EmittedEvent) -> Result<Self, Self::Error> {
                            Self::try_from_event(event.from_address, event.keys, event.data)
                        }
                    }

                    impl TryFrom<#snrs_types::Event> for #event_name_str {
                        type Error = String;

                        fn try_from(event: #snrs_types::Event) -> Result<Self, Self::Error> {
                            Self::try_from_event(event.from_address, event.keys, event.data)
                        }
                    }
                };

                vec![ExpansionResult::new(&full_path).with_item(&event_name, definition)]
            }
            cainome_parser::tokens::EventKind::Struct => {
                let fields = [self.keys.clone(), self.data.clone()].concat();

                let ccs = utils::str_to_type(&ctx.cainome_serde_path);

                let ctx = ExpansionContextFactory::from(ctx)
                    .with_derives(get_additional_derive_requirements(&fields, ctx))
                    .build();

                let declaration = struct_declaration(
                    &full_path,
                    &event_name,
                    &fields,
                    &vec![],
                    &HashMap::new(),
                    &ctx,
                );
                let implementation = struct_implementation(
                    &full_path,
                    &event_name,
                    &fields,
                    &vec![],
                    &HashMap::new(),
                    &ctx,
                );

                let definition = quote! {
                    #declaration

                    #implementation

                    impl #event_name_str {
                        pub fn event_selector() -> #snrs_types::Felt {
                            #snrs_utils::get_selector_from_name(#event_name).unwrap()
                        }

                        pub fn event_name() -> &'static str {
                            #event_name
                        }

                        pub(crate) fn try_from_event(from_address: starknet::core::types::Felt, keys: Vec<starknet::core::types::Felt>, data: Vec<starknet::core::types::Felt>) -> Result<Self, String> {
                            if keys.is_empty() {
                                return Err("Event has no key".to_string());
                            }

                            use #ccs::CairoSerde;

                            if keys[0] == #snrs_utils::get_selector_from_name(#event_name).unwrap_or_else(|_| panic!("Invalid selector for {}", #event_name)) {
                                let res = #event_name_str::cairo_deserialize(&data, 0)
                                    .map_err(|e| format!("Could not deserialize {} event data: {:?}", #event_name, e))?;

                                return Ok(res);
                            };


                            Err(format!("Could not match any event from keys {:?}", keys))
                        }
                    }

                    impl TryFrom<#snrs_types::EmittedEvent> for #event_name_str {
                        type Error = String;

                        fn try_from(event: #snrs_types::EmittedEvent) -> Result<Self, Self::Error> {
                            Self::try_from_event(event.from_address, event.keys, event.data)
                        }
                    }

                    impl TryFrom<#snrs_types::Event> for #event_name_str {
                        type Error = String;

                        fn try_from(event: #snrs_types::Event) -> Result<Self, Self::Error> {
                            Self::try_from_event(event.from_address, event.keys, event.data)
                        }
                    }
                };

                vec![ExpansionResult::new(&full_path).with_item(&event_name, definition)]
            }
        }
    }
}
