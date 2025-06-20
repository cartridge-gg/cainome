use async_trait::async_trait;
use convert_case::{Case, Casing};
use std::collections::HashMap;

use cainome_parser::tokens::{
    Composite, CompositeInnerKind, CompositeType, Function, StateMutability, Token,
};

use crate::error::CainomeCliResult;
use crate::plugins::builtins::BuiltinPlugin;
use crate::plugins::PluginInput;

pub struct GolangPlugin;

impl GolangPlugin {
    pub fn new() -> Self {
        Self {}
    }

    /// Maps a Cairo core basic type to its Go equivalent
    fn map_core_basic_type(&self, cairo_type: &str) -> String {
        match cairo_type {
            "felt" | "core::felt252" => "*felt.Felt".to_string(),
            "core::bool" => "bool".to_string(),
            "core::integer::u8" => "uint8".to_string(),
            "core::integer::u16" => "uint16".to_string(),
            "core::integer::u32" => "uint32".to_string(),
            "core::integer::u64" => "uint64".to_string(),
            "core::integer::u128" => "*big.Int".to_string(),
            "core::integer::usize" => "uint64".to_string(),
            "core::integer::i8" => "int8".to_string(),
            "core::integer::i16" => "int16".to_string(),
            "core::integer::i32" => "int32".to_string(),
            "core::integer::i64" => "int64".to_string(),
            "core::integer::i128" => "*big.Int".to_string(),
            "core::starknet::contract_address::ContractAddress" => "*felt.Felt".to_string(),
            "core::starknet::class_hash::ClassHash" => "*felt.Felt".to_string(),
            "core::bytes_31::bytes31" => "[31]byte".to_string(),
            "()" => "struct{}".to_string(),
            _ => format!("interface{{}} // Unknown type: {}", cairo_type),
        }
    }

    /// Maps a Cairo composite builtin type to its Go equivalent
    fn map_composite_builtin_type(&self, cairo_type: &str) -> String {
        match cairo_type {
            "core::byte_array::ByteArray" => "[]byte".to_string(),
            "core::starknet::eth_address::EthAddress" => "[20]byte".to_string(),
            "core::integer::u256" => "*big.Int".to_string(),
            _ => format!("interface{{}} // Unknown composite builtin: {}", cairo_type),
        }
    }

    /// Converts a token to its Go type representation
    fn token_to_go_type(&self, token: &Token) -> String {
        match token {
            Token::CoreBasic(core_basic) => self.map_core_basic_type(&core_basic.type_path),
            Token::Array(array) => {
                let inner_type = self.token_to_go_type(&array.inner);
                format!("[]{}", inner_type)
            }
            Token::Tuple(tuple) => {
                if tuple.inners.is_empty() {
                    "struct{}".to_string()
                } else {
                    let field_types: Vec<String> = tuple
                        .inners
                        .iter()
                        .enumerate()
                        .map(|(i, token)| format!("Field{} {}", i, self.token_to_go_type(token)))
                        .collect();
                    format!("struct {{\n\t{}\n}}", field_types.join("\n\t"))
                }
            }
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    self.map_composite_builtin_type(&composite.type_path_no_generic())
                } else {
                    composite.type_name().to_case(Case::Pascal)
                }
            }
            Token::Option(option) => {
                let inner_type = self.token_to_go_type(&option.inner);
                format!("*{}", inner_type) // Use pointer for optional types
            }
            Token::Result(result) => {
                let ok_type = self.token_to_go_type(&result.inner);
                let err_type = self.token_to_go_type(&result.error);
                format!("Result[{}, {}]", ok_type, err_type)
            }
            Token::NonZero(non_zero) => self.token_to_go_type(&non_zero.inner),
            Token::Function(_) => "func".to_string(),
        }
    }

    /// Generates Go struct definition for a Cairo composite type
    fn generate_struct(&self, composite: &Composite) -> String {
        let struct_name = composite.type_name().to_case(Case::Pascal);
        let mut struct_def = format!("type {} struct {{\n", struct_name);

        for inner in &composite.inners {
            let field_name = inner.name.to_case(Case::Pascal);
            let field_type = self.token_to_go_type(&inner.token);
            let json_tag = format!("`json:\"{}\"`", inner.name);
            struct_def.push_str(&format!("\t{} {} {}\n", field_name, field_type, json_tag));
        }

        struct_def.push_str("}\n");
        struct_def
    }

    /// Generates Go enum definition for a Cairo enum type
    fn generate_enum(&self, composite: &Composite) -> String {
        let enum_name = composite.type_name().to_case(Case::Pascal);
        let mut enum_def = String::new();

        // Generate the enum type
        enum_def.push_str(&format!("type {} struct {{\n", enum_name));
        enum_def.push_str("\tVariant string `json:\"variant\"`\n");
        enum_def.push_str("\tValue   interface{} `json:\"value,omitempty\"`\n");
        enum_def.push_str("}\n\n");

        // Generate constants for each variant
        enum_def.push_str("const (\n");
        for inner in &composite.inners {
            let variant_name = format!("{}_{}", enum_name, inner.name.to_case(Case::Pascal));
            enum_def.push_str(&format!("\t{} = \"{}\"\n", variant_name, inner.name));
        }
        enum_def.push_str(")\n\n");

        // Generate constructor functions for each variant
        for inner in &composite.inners {
            let variant_name = inner.name.to_case(Case::Pascal);
            let constructor_name = format!("New{}{}", enum_name, variant_name);

            match inner.kind {
                CompositeInnerKind::NotUsed => {
                    // Unit variant (no data)
                    enum_def.push_str(&format!(
                        "func {}() {} {{\n\treturn {} {{\n\t\tVariant: \"{}\",\n\t}}\n}}\n\n",
                        constructor_name, enum_name, enum_name, inner.name
                    ));
                }
                CompositeInnerKind::Data => {
                    // Data variant
                    let data_type = self.token_to_go_type(&inner.token);
                    enum_def.push_str(&format!(
                        "func {}(value {}) {} {{\n\treturn {} {{\n\t\tVariant: \"{}\",\n\t\tValue: value,\n\t}}\n}}\n\n",
                        constructor_name, data_type, enum_name, enum_name, inner.name
                    ));
                }
                _ => {}
            }
        }

        enum_def
    }

    /// Generates Go function definition for a Cairo contract function
    fn generate_function(&self, function: &Function, contract_name: &str) -> String {
        let func_name = function.name.to_case(Case::Pascal);
        let receiver_name = contract_name.to_case(Case::Snake);

        // Generate parameters
        let mut params = Vec::new();
        for (param_name, param_token) in &function.inputs {
            let go_type = self.token_to_go_type(param_token);
            params.push(format!("{} {}", param_name.to_case(Case::Snake), go_type));
        }

        // Generate return types
        let mut returns = Vec::new();
        for (i, output_token) in function.outputs.iter().enumerate() {
            let go_type = self.token_to_go_type(output_token);
            if function.outputs.len() == 1 {
                returns.push(go_type);
            } else {
                returns.push(format!("ret{} {}", i, go_type));
            }
        }

        // Add error return
        returns.push("error".to_string());

        let return_str = if returns.len() == 1 {
            returns[0].clone()
        } else {
            format!("({})", returns.join(", "))
        };

        let is_view = function.state_mutability == StateMutability::View;
        let method_type = if is_view { "Call" } else { "Invoke" };

        let mut func_def = format!(
            "func ({} *{}) {}({}) {} {{\n",
            receiver_name,
            contract_name.to_case(Case::Pascal),
            func_name,
            params.join(", "),
            return_str
        );

        func_def.push_str(&format!(
            "\t// TODO: Implement {} method for {}\n",
            method_type, func_name
        ));
        func_def.push_str("\tpanic(\"not implemented\")\n");
        func_def.push_str("}\n\n");

        func_def
    }

    /// Generates the main contract struct and constructor
    fn generate_contract(&self, contract_name: &str, functions: &[&Function]) -> String {
        let struct_name = contract_name.to_case(Case::Pascal);
        let mut contract_def = String::new();

        // Generate contract struct
        contract_def.push_str(&format!("type {} struct {{\n", struct_name));
        contract_def.push_str("\tcontractAddress *felt.Felt\n");
        contract_def.push_str("\tprovider Provider // Interface for StarkNet provider\n");
        contract_def.push_str("}\n\n");

        // Generate constructor
        contract_def.push_str(&format!(
            "func New{}(contractAddress *felt.Felt, provider Provider) *{} {{\n",
            struct_name, struct_name
        ));
        contract_def.push_str(&format!("\treturn &{} {{\n", struct_name));
        contract_def.push_str("\t\tcontractAddress: contractAddress,\n");
        contract_def.push_str("\t\tprovider: provider,\n");
        contract_def.push_str("\t}\n");
        contract_def.push_str("}\n\n");

        // Generate methods for each function
        for function in functions {
            contract_def.push_str(&self.generate_function(function, contract_name));
        }

        contract_def
    }

    /// Generates the Go package header with imports
    fn generate_package_header(&self, package_name: &str) -> String {
        format!(
            r#"// Code generated by Cainome. DO NOT EDIT.
// Generated from ABI file.

package {}

import (
	"math/big"
	"github.com/NethermindEth/juno/core/felt"
)

// Provider interface for StarkNet interactions
type Provider interface {{
	Call(contractAddress *felt.Felt, selector *felt.Felt, calldata []*felt.Felt) ([]*felt.Felt, error)
	Invoke(contractAddress *felt.Felt, selector *felt.Felt, calldata []*felt.Felt) (string, error)
}}

"#,
            package_name
        )
    }
}

#[async_trait]
impl BuiltinPlugin for GolangPlugin {
    async fn generate_code(&self, input: &PluginInput) -> CainomeCliResult<()> {
        tracing::trace!("Golang plugin requested");

        for contract in &input.contracts {
            let contract_name = contract
                .name
                .split("::")
                .last()
                .unwrap_or(&contract.name)
                .from_case(Case::Snake)
                .to_case(Case::Pascal);

            let package_name = contract_name.to_case(Case::Snake);
            let mut generated_code = self.generate_package_header(&package_name);

            // Collect all composite types (structs and enums) and functions
            let mut composites = HashMap::new();
            let mut functions: Vec<&Function> = Vec::new();

            // Process structs
            for token in &contract.tokens.structs {
                if let Token::Composite(composite) = token {
                    if !composite.is_builtin() {
                        composites.insert(composite.type_path_no_generic(), composite);
                    }
                }
            }

            // Process enums
            for token in &contract.tokens.enums {
                if let Token::Composite(composite) = token {
                    if !composite.is_builtin() {
                        composites.insert(composite.type_path_no_generic(), composite);
                    }
                }
            }

            // Process standalone functions
            for token in &contract.tokens.functions {
                if let Token::Function(function) = token {
                    functions.push(function);
                }
            }

            // Process interface functions
            for interface_functions in contract.tokens.interfaces.values() {
                for token in interface_functions {
                    if let Token::Function(function) = token {
                        functions.push(function);
                    }
                }
            }

            // Generate composite types (structs and enums)
            for composite in composites.values() {
                match composite.r#type {
                    CompositeType::Struct => {
                        generated_code.push_str(&self.generate_struct(composite));
                        generated_code.push('\n');
                    }
                    CompositeType::Enum => {
                        generated_code.push_str(&self.generate_enum(composite));
                        generated_code.push('\n');
                    }
                    _ => {}
                }
            }

            // Generate contract struct and methods
            generated_code.push_str(&self.generate_contract(&contract_name, &functions));

            // Write to file
            let filename = format!("{}.go", package_name);
            let mut out_path = input.output_dir.clone();
            out_path.push(filename);

            tracing::trace!("Golang writing file {}", out_path);
            std::fs::write(&out_path, generated_code)?;
        }

        Ok(())
    }
}
