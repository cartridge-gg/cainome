use cainome_parser::{
    tokens::{Function, NamedToken, Token},
    TypeRegistry,
};
use proc_macro2::TokenStream;
use syn::parse_quote;

use crate::expand::{
    contract::Contract, for_tests::assert_code_has, Expandable, ExpansionContextFactory, Module,
};

#[test]
fn test_naive_contract_expansion() {
    let ctx = ExpansionContextFactory::new("ContractName").build();

    let contract = Contract {
        name: "ContractName".to_string(),
        derives: vec![],
        readonly_methods: vec![],
        mutating_methods: vec![],
        constructor: None,
    };

    let generated = Module::new()
        .with_includes(contract.expand(&ctx))
        .to_token_stream();

    let expected: TokenStream = parse_quote! {
        pub struct ContractName<A: starknet::accounts::ConnectedAccount + Sync> {
            pub address: starknet::core::types::Felt,
            pub account: A,
            pub block_id: starknet::core::types::BlockId,
        }
    };

    assert_code_has(&generated, &expected, "Contract struct not found");

    let expected: TokenStream = parse_quote! {
        pub struct ContractNameReader<P: starknet::providers::Provider + Sync> {
            pub address: starknet::core::types::Felt,
            pub provider: P,
            pub block_id: starknet::core::types::BlockId,
        }
    };

    assert_code_has(&generated, &expected, "Contract reader not found");
}

#[test]
fn test_naive_contract_with_view_function() {
    let mut registry = TypeRegistry::new();

    let function = Token::Function(Function {
        name: "get_value".to_string(),
        inputs: vec![NamedToken {
            name: "in1".to_string(),
            token: registry.get("felt").unwrap(),
        }],
        outputs: vec![registry.get("felt").unwrap()],
        state_mutability: cainome_parser::tokens::StateMutability::View,
        named_outputs: vec![],
    });

    registry.set("some_unique_path", function);

    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let contract = Contract::new("ContractName", vec![], &registry);

    let generated = Module::new()
        .with_includes(contract.expand(&ctx))
        .to_token_stream();

    let expected: TokenStream = parse_quote! {
        #[allow(clippy::ptr_arg)]
        #[allow(clippy::too_many_arguments)]
        pub fn get_value(
            &self,
            in1: &starknet::core::types::Felt,
        ) -> cainome::cairo_serde::call::FCall<A::Provider, starknet::core::types::Felt> {
            use cainome::cairo_serde::CairoSerde;
            let mut __calldata = vec![];
            __calldata.extend(starknet::core::types::Felt::cairo_serialize(in1));
            let __call = starknet::core::types::FunctionCall {
                contract_address: self.address,
                entry_point_selector: starknet::macros::selector!("get_value"),
                calldata: __calldata,
            };
            cainome::cairo_serde::call::FCall::new(__call, self.provider())
        }
    };

    assert_code_has(&generated, &expected, "Contract class not found");

    let expected: TokenStream = parse_quote! {
        #[allow(clippy::ptr_arg)]
        #[allow(clippy::too_many_arguments)]
        pub fn get_value(
            &self,
            in1: &starknet::core::types::Felt,
        ) -> cainome::cairo_serde::call::FCall<P, starknet::core::types::Felt> {
            use cainome::cairo_serde::CairoSerde;
            let mut __calldata = vec![];
            __calldata.extend(starknet::core::types::Felt::cairo_serialize(in1));
            let __call = starknet::core::types::FunctionCall {
                contract_address: self.address,
                entry_point_selector: starknet::macros::selector!("get_value"),
                calldata: __calldata,
            };
            cainome::cairo_serde::call::FCall::new(__call, self.provider())
        }
    };

    assert_code_has(&generated, &expected, "Contract reader not found");
}

#[test]
fn test_naive_contract_with_view_mutating_function() {
    let mut registry = TypeRegistry::new();

    let function = Token::Function(Function {
        name: "get_value".to_string(),
        inputs: vec![NamedToken {
            name: "in1".to_string(),
            token: registry.get("felt").unwrap(),
        }],
        outputs: vec![registry.get("felt").unwrap()],
        state_mutability: cainome_parser::tokens::StateMutability::External,
        named_outputs: vec![],
    });

    registry.set("some_unique_path", function);

    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let contract = Contract::new("ContractName", vec![], &registry);

    let generated = Module::new()
        .with_includes(contract.expand(&ctx))
        .to_token_stream();

    let expected: TokenStream = parse_quote! {
        #[allow(clippy::ptr_arg)]
        #[allow(clippy::too_many_arguments)]
        pub fn get_value_getcall(
            &self,
            in1: &starknet::core::types::Felt,
        ) -> starknet::core::types::Call {
            use cainome::cairo_serde::CairoSerde;
            let mut __calldata = vec![];
            __calldata.extend(starknet::core::types::Felt::cairo_serialize(in1));
            starknet::core::types::Call {
                to: self.address,
                selector: starknet::macros::selector!("get_value"),
                calldata: __calldata,
            }
        }
    };

    assert_code_has(&generated, &expected, "Contract method not found");

    let expected: TokenStream = parse_quote! {
        #[allow(clippy::ptr_arg)]
        #[allow(clippy::too_many_arguments)]
        pub fn get_value(
            &self,
            in1: &starknet::core::types::Felt,
        ) -> starknet::accounts::ExecutionV3<A> {
            let __call = self.get_value_getcall(in1);
            self.account.execute_v3(vec![__call])
        }
    };

    assert_code_has(&generated, &expected, "Contract method not found");
}
