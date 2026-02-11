use cainome_parser::{
    tokens::{Event, EventKind, NamedToken, Token},
    TypeRegistry,
};

use proc_macro2::TokenStream;
use syn::parse_quote;

use crate::expand::{for_tests::assert_code_has, Expandable, ExpansionContextFactory, Module};

#[test]
fn test_struct_event_expansion() {
    let mut registry = TypeRegistry::new();

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

    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(event.expand(&ctx).unwrap())
        .unwrap()
        .token_stream();

    let expected = parse_quote! {
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
    let mut registry = fixtures::simple_nested_struct_enum();

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_root_module_path("crate")
        .build();

    registry.apply_substitutions(&ctx.substitutions);

    let token = registry
        .get("contracts::abicov::events::events::Event")
        .unwrap();

    let Token::Event(enum_event) = &*token.borrow() else {
        panic!("Token should be an Event. Something is wrong with fixture.");
    };

    let generated = Module::new()
        .with_includes(enum_event.expand(&ctx).unwrap())
        .unwrap()
        .token_stream();

    let expected: TokenStream = parse_quote! {
        pub enum Event {
            Simple(crate::contracts::abicov::events::events::SimpleEvent)
        }
    };

    assert_code_has(&generated, &expected, "Event not found");
}
