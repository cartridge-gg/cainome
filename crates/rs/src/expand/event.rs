use crate::expand::{
    enumeration::{enum_declaration, enum_implementation},
    structure::{struct_declaration, struct_implementation},
    types::CairoToRust,
    utils, Expandable, ExpansionContext,
};
use cainome_parser::tokens::{Event, EventKind, Token};
use proc_macro2::TokenStream;
use quote::quote;

fn from_event_conversion_from_enum(event: &Event, ctx: &ExpansionContext) -> TokenStream {
    let event_name_str = event.type_name();

    let event_name = utils::str_to_ident(&event_name_str);

    let snrs_utils = utils::snrs_utils();
    let ccs = utils::cainome_cairo_serde();

    let mut variants: Vec<TokenStream> = vec![];

    for inner in event.flat.iter() {
        let variant_name_str = utils::str_to_litstr(&inner.name);
        let variant_ident = utils::str_to_ident(&inner.name);

        let inner_token = &*inner.token.borrow();

        let Token::Event(event) = inner_token else {
            unreachable!("Flat event variant is always an event");
        };

        let inner_type_name = inner_token.to_rust_type();

        let inner_type_name_id = utils::str_to_ident(&inner_type_name);

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
        let inner_type_name = inner_token.to_rust_type();

        let inner_type_name_str = utils::str_to_litstr(&inner_type_name);
        let inner_type_name_id = utils::str_to_ident(&inner_type_name);

        let Token::Event(Event { kind, .. }) = inner_token else {
            unreachable!("Nested event variant is always an event");
        };

        let value = match kind {
            EventKind::Struct => quote! {
                use #ccs::CairoSerde;

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
        #(#variants)*
    }
}

impl Expandable for Event {
    fn expand(&self, expansion_context: &ExpansionContext) -> TokenStream {
        let type_name = self.type_name();
        let event_name = utils::str_to_ident(&type_name);

        // Generate type definition, struct or enum, depending on the event type.
        match self.kind {
            cainome_parser::tokens::EventKind::Enum => {
                let snrs_types = utils::snrs_types();

                let event_conversion_implementation =
                    from_event_conversion_from_enum(self, expansion_context);

                let variants = [self.nested.clone(), self.flat.clone()].concat();

                let declaration = enum_declaration(&type_name, &variants, expansion_context);
                let implementation = enum_implementation(&type_name, &variants, expansion_context);

                quote! {

                    #declaration

                    #implementation

                    impl #event_name {
                        pub(crate) fn try_from_event(from_address: starknet::core::types::Felt, keys: Vec<starknet::core::types::Felt>, data: Vec<starknet::core::types::Felt>) -> Result<Self, String> {
                            if keys.is_empty() {
                                return Err("Event has no key".to_string());
                            }

                            #event_conversion_implementation

                            Err(format!("Could not match any event from keys {:?}", keys))
                        }
                    }

                    impl TryFrom<#snrs_types::EmittedEvent> for #event_name {
                        type Error = String;

                        fn try_from(event: #snrs_types::EmittedEvent) -> Result<Self, Self::Error> {
                            Self::try_from_event(event.from_address, event.keys, event.data)
                        }
                    }

                    impl TryFrom<#snrs_types::Event> for #event_name {
                        type Error = String;

                        fn try_from(event: #snrs_types::Event) -> Result<Self, Self::Error> {
                            Self::try_from_event(event.from_address, event.keys, event.data)
                        }
                    }
                }
            }
            cainome_parser::tokens::EventKind::Struct => {
                let fields = [self.keys.clone(), self.data.clone()].concat();

                let declaration = struct_declaration(&type_name, &fields, expansion_context);
                let implementation = struct_implementation(&type_name, &fields, expansion_context);

                quote! {
                    #declaration

                    #implementation
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cainome_parser::{
        tokens::{Event, EventKind, NamedToken, Token},
        TypeRegistry,
    };
    use proc_macro2::TokenStream;
    use quote::quote;
    use quote::ToTokens;
    use syn::{parse_quote, ItemEnum, ItemStruct};

    use crate::expand::{Expandable, ExpansionContext};

    fn assert_code_has<T: ToTokens>(generated: &TokenStream, expected: &T, message: &str) {
        let file: syn::File = syn::parse2(generated.clone()).expect("expected file-like tokens");

        let expected_str = expected.to_token_stream().to_string();

        let has_match = file.items.iter().any(|item| {
            let item = item.to_token_stream().to_string();
            item == expected_str
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
    fn test_struct_event_expansion() {
        let registry = TypeRegistry::new();

        let event = Event {
            kind: EventKind::Struct,
            keys: vec![],
            data: vec![NamedToken {
                name: "data".to_string(),
                token: registry.get("core::felt252").unwrap(),
            }],
            type_path: "contracts::abicov::events::events::SimpleEvent".to_string(),
            nested: vec![],
            flat: vec![],
            generic_args: vec![],
        };

        let ctx = ExpansionContext::new("ContractName");
        let generated = event.expand(&ctx);

        let expected: ItemStruct = parse_quote! {
            pub struct SimpleEvent {
                pub data: starknet::core::types::Felt
            }
        };

        assert_code_has(&generated, &expected, "Event not found");
    }

    mod fixtures {
        use cainome_parser::{tokens::*, TypeRegistry};

        pub fn simple_nested_struct_enum() -> TypeRegistry {
            let mut registry = TypeRegistry::new();
            registry.set(
                "contracts::abicov::events::events::SimpleEvent",
                Token::Event(Event {
                    kind: EventKind::Struct,
                    keys: vec![],
                    data: vec![NamedToken {
                        name: "data".to_string(),
                        token: registry.get("core::felt252").unwrap(),
                    }],
                    type_path: "contracts::abicov::events::events::SimpleEvent".to_string(),
                    nested: vec![],
                    flat: vec![],
                    generic_args: vec![],
                }),
            );

            registry.set(
                "contracts::abicov::events::events::Event",
                Token::Event(Event {
                    kind: EventKind::Enum,
                    type_path: "contracts::abicov::events::events::Event".to_string(),
                    keys: vec![],
                    data: vec![],
                    nested: vec![NamedToken {
                        name: "Simple".to_string(),
                        token: registry
                            .get("contracts::abicov::events::events::SimpleEvent")
                            .unwrap(),
                    }],
                    flat: vec![],
                    generic_args: vec![],
                }),
            );

            registry
        }

        pub fn simple_nested_enum_enum() -> TypeRegistry {
            let mut registry = TypeRegistry::new();

            registry.set(
                "contracts::abicov::events::events::SimpleEvent",
                Token::Event(Event {
                    kind: EventKind::Struct,
                    type_path: "contracts::abicov::events::events::SimpleEvent".to_string(),
                    keys: vec![],
                    data: vec![NamedToken {
                        name: "data".to_string(),
                        token: registry.get("core::felt252").unwrap(),
                    }],
                    nested: vec![],
                    flat: vec![],
                    generic_args: vec![],
                }),
            );

            registry.set(
                "contracts::abicov::events::events::NestedEnum",
                Token::Event(Event {
                    kind: EventKind::Enum,
                    type_path: "contracts::abicov::events::events::NestedEnum".to_string(),
                    keys: vec![],
                    data: vec![],
                    nested: vec![NamedToken {
                        name: "Variant1".to_string(),
                        token: registry
                            .get("contracts::abicov::events::events::SimpleEvent")
                            .unwrap(),
                    }],
                    flat: vec![],
                    generic_args: vec![],
                }),
            );

            registry.set(
                "contracts::abicov::events::events::Event",
                Token::Event(Event {
                    kind: EventKind::Enum,
                    type_path: "contracts::abicov::events::events::Event".to_string(),
                    keys: vec![],
                    data: vec![],
                    nested: vec![NamedToken {
                        name: "Simple".to_string(),
                        token: registry
                            .get("contracts::abicov::events::events::NestedEnum")
                            .unwrap(),
                    }],
                    flat: vec![],
                    generic_args: vec![],
                }),
            );

            registry
        }

        pub fn simple_nested_flat_enum() -> TypeRegistry {
            let mut registry = TypeRegistry::new();

            registry.set(
                "contracts::abicov::events::events::SimpleEvent",
                Token::Event(Event {
                    kind: EventKind::Struct,
                    type_path: "contracts::abicov::events::events::SimpleEvent".to_string(),
                    keys: vec![],
                    data: vec![NamedToken {
                        name: "data".to_string(),
                        token: registry.get("core::felt252").unwrap(),
                    }],
                    nested: vec![],
                    flat: vec![],
                    generic_args: vec![],
                }),
            );

            registry.set(
                "contracts::abicov::events::events::NestedEnum",
                Token::Event(Event {
                    kind: EventKind::Enum,
                    type_path: "contracts::abicov::events::events::NestedEnum".to_string(),
                    keys: vec![],
                    data: vec![],
                    nested: vec![NamedToken {
                        name: "Variant1".to_string(),
                        token: registry
                            .get("contracts::abicov::events::events::SimpleEvent")
                            .unwrap(),
                    }],
                    flat: vec![],
                    generic_args: vec![],
                }),
            );

            registry.set(
                "contracts::abicov::events::events::Event",
                Token::Event(Event {
                    kind: EventKind::Enum,
                    type_path: "contracts::abicov::events::events::Event".to_string(),
                    keys: vec![],
                    data: vec![],
                    nested: vec![],
                    flat: vec![NamedToken {
                        name: "Simple".to_string(),
                        token: registry
                            .get("contracts::abicov::events::events::NestedEnum")
                            .unwrap(),
                    }],
                    generic_args: vec![],
                }),
            );

            registry
        }
    }

    #[test]
    fn test_simple_case_nested_struct_in_enum() {
        let registry = fixtures::simple_nested_struct_enum();
        let token = registry
            .get("contracts::abicov::events::events::Event")
            .unwrap();

        let Token::Event(enum_event) = &*token.borrow() else {
            panic!("Token should be an Event. Something is wrong with fixture.");
        };

        let ctx = ExpansionContext::new("ContractName");
        let generated = enum_event.expand(&ctx);

        let expected: ItemEnum = parse_quote! {
            pub enum Event {
                Simple(SimpleEvent)
            }
        };

        assert_code_has(&generated, &expected, "Event not found");
    }

    #[test]
    fn expand_whole_file() {
        let registry = fixtures::simple_nested_flat_enum();

        let ctx = ExpansionContext::new("ContractName");
        let mut generated = vec![];
        for val in registry.values().iter() {
            let Token::Event(event) = &*val.borrow() else {
                continue;
            };

            generated.push(event.expand(&ctx));
        }

        let combined = quote! {
            mod cainome {
                pub use cainome_cairo_serde as cairo_serde;
            }

            #(#generated)*
        };

        println!("{}", combined.to_string());
    }
}
