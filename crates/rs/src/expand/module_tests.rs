use cainome_parser::{
    tokens::{Struct, Token},
    TypeRegistry,
};

use crate::expand::{
    for_tests::assert_code_has, Expandable, ExpansionContext, ExpansionContextFactory,
    ExpansionResult, Module,
};

#[test]
fn test_simple_nested_module_expand_no_content() {
    let results =
        vec![ExpansionResult::new("module1::sub1::TypeA").with_item("some", quote::quote! {})];

    let module = Module::new().with_includes(results);

    assert_eq!(module.name, "");

    assert!(module.submodules.contains_key("module1"));
    assert_eq!(module.submodules["module1"].name, "module1");
    assert_eq!(module.submodules["module1"].content.len(), 0);

    assert!(module.submodules["module1"].submodules.contains_key("sub1"));
    assert_eq!(module.submodules["module1"].submodules["sub1"].name, "sub1");
    assert_eq!(
        module.submodules["module1"].submodules["sub1"]
            .content
            .len(),
        1
    );

    let generated = module.to_token_stream();

    let expected = quote::quote! {
        pub mod module1 { pub mod sub1 { } }
    };

    assert_code_has(&generated, &expected, "Incorrect module structure");
}

#[test]
fn test_2_nested_modules_with_common_parent_expand_no_content() {
    let results = vec![
        ExpansionResult::new("module1::sub1::TypeA").with_item("some", quote::quote! {}),
        ExpansionResult::new("module1::sub2::TypeB").with_item("some", quote::quote! {}),
    ];

    let module = Module::new().with_includes(results);

    let generated = module.to_token_stream();

    let expected = quote::quote! {
        pub mod module1 { pub mod sub1 { } pub mod sub2 { } }
    };

    assert_code_has(&generated, &expected, "Incorrect module structure");
}

#[test]
fn test_2_nested_modules_with_struct_and_reference() {
    let mut registry = TypeRegistry::new();
    let ctx = ExpansionContextFactory::new("ContractName").build();

    registry.apply_substitutions(&ctx.substitutions);

    let s1 = Struct::new("module1::sub1::TypeA", &registry)
        .unwrap()
        .with_field("f1", registry.get("felt").unwrap());

    registry.set("module1::sub1::TypeA", Token::Struct(s1));

    let s2 = Struct::new("module1::sub2::TypeB", &registry)
        .unwrap()
        .with_field("f1", registry.get("felt").unwrap())
        .with_field("f2", registry.get("module1::sub1::TypeA").unwrap());

    registry.set("module1::sub2::TypeB", Token::Struct(s2));

    let mut root = Module::new();

    for token_ref in registry.values() {
        let Token::Struct(structure) = &*token_ref.borrow() else {
            continue;
        };
        root.include_many(structure.expand(&ctx));
    }

    let generated = root.to_token_stream();

    let expected = vec![
        quote::quote! {
            __size += crate::module1::sub1::TypeA::cairo_serialized_size(&__rust.f2);
        },
        quote::quote! {
            let f2 = crate::module1::sub1::TypeA::cairo_deserialize(__felts, __offset)?;
            __offset += crate::module1::sub1::TypeA::cairo_serialized_size(&f2);
        },
        quote::quote! {
            pub struct TypeB {
                pub f1: starknet::core::types::Felt,
                pub f2: crate::module1::sub1::TypeA
            }
        },
    ];

    for item in expected {
        assert_code_has(&generated, &item, "Incorrect module structure");
    }
}
