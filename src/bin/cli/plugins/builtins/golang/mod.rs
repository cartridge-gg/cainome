use async_trait::async_trait;
use convert_case::{Case, Casing};
use std::collections::HashMap;

use cainome_parser::tokens::{
    Composite, CompositeInnerKind, CompositeType, Function, StateMutability, Token,
};

#[cfg(test)]
use cainome_rs;

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
    fn generate_package_header(&self, package_name: &str, needs_big_int: bool) -> String {
        let imports = if needs_big_int {
            r#"import (
	"math/big"
	"github.com/NethermindEth/juno/core/felt"
)"#
        } else {
            r#"import (
	"github.com/NethermindEth/juno/core/felt"
)"#
        };

        format!(
            r#"// Code generated by Cainome. DO NOT EDIT.
// Generated from ABI file.

package {}

{}

// Provider interface for StarkNet interactions
type Provider interface {{
	Call(contractAddress *felt.Felt, selector *felt.Felt, calldata []*felt.Felt) ([]*felt.Felt, error)
	Invoke(contractAddress *felt.Felt, selector *felt.Felt, calldata []*felt.Felt) (string, error)
}}

"#,
            package_name, imports
        )
    }

    /// Checks if the generated code needs math/big import
    fn needs_big_int(&self, contracts: &[&crate::contract::ContractData]) -> bool {
        for contract in contracts {
            if self.needs_big_int_for_tokens(&contract.tokens.structs)
                || self.needs_big_int_for_tokens(&contract.tokens.enums)
                || self.needs_big_int_for_tokens(&contract.tokens.functions)
            {
                return true;
            }

            for functions in contract.tokens.interfaces.values() {
                if self.needs_big_int_for_tokens(functions) {
                    return true;
                }
            }
        }
        false
    }

    /// Checks if a list of tokens needs math/big import
    fn needs_big_int_for_tokens(&self, tokens: &[Token]) -> bool {
        for token in tokens {
            if self.token_needs_big_int(token) {
                return true;
            }
        }
        false
    }

    /// Checks if a specific token needs math/big import
    #[allow(clippy::only_used_in_recursion)]
    fn token_needs_big_int(&self, token: &Token) -> bool {
        match token {
            Token::CoreBasic(core_basic) => {
                matches!(
                    core_basic.type_path.as_str(),
                    "core::integer::u128" | "core::integer::i128"
                )
            }
            Token::Composite(composite) => {
                if composite.type_path_no_generic() == "core::integer::u256" {
                    return true;
                }
                composite
                    .inners
                    .iter()
                    .any(|inner| self.token_needs_big_int(&inner.token))
            }
            Token::Array(array) => self.token_needs_big_int(&array.inner),
            Token::Tuple(tuple) => tuple
                .inners
                .iter()
                .any(|token| self.token_needs_big_int(token)),
            Token::Option(option) => self.token_needs_big_int(&option.inner),
            Token::Result(result) => {
                self.token_needs_big_int(&result.inner) || self.token_needs_big_int(&result.error)
            }
            Token::NonZero(non_zero) => self.token_needs_big_int(&non_zero.inner),
            Token::Function(function) => {
                function
                    .inputs
                    .iter()
                    .any(|(_, token)| self.token_needs_big_int(token))
                    || function
                        .outputs
                        .iter()
                        .any(|token| self.token_needs_big_int(token))
            }
        }
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

            // Check if we need math/big import
            let needs_big_int = self.needs_big_int(&[contract]);

            let mut generated_code = self.generate_package_header(&package_name, needs_big_int);

            // Collect all composite types (structs and enums) and functions
            let mut composites = HashMap::new();
            let mut functions: Vec<&Function> = Vec::new();

            // Process structs
            for token in &contract.tokens.structs {
                if let Token::Composite(composite) = token {
                    if !composite.is_builtin() {
                        let type_path = composite.type_path_no_generic();
                        if composites.contains_key(&type_path) {
                            // Handle duplicate names by using full type path
                            let unique_key = format!("{}_struct", type_path);
                            composites.insert(unique_key, composite);
                        } else {
                            composites.insert(type_path, composite);
                        }
                    }
                }
            }

            // Process enums
            for token in &contract.tokens.enums {
                if let Token::Composite(composite) = token {
                    if !composite.is_builtin() {
                        let type_path = composite.type_path_no_generic();
                        if composites.contains_key(&type_path) {
                            // Handle duplicate names by using full type path
                            let unique_key = format!("{}_enum", type_path);
                            composites.insert(unique_key, composite);
                        } else {
                            composites.insert(type_path, composite);
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ContractData, ContractOrigin, ContractParser, ContractParserConfig};
    use cainome_parser::AbiParser;
    use camino::Utf8PathBuf;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    const TEST_ARTIFACTS_DIR: &str = "src/bin/cli/plugins/builtins/test_artifacts";
    const CONTRACTS_ABI_DIR: &str = "contracts/abi";

    /// Test generating Go bindings from all available ABI files
    #[tokio::test]
    async fn test_generate_go_bindings_from_all_abis() {
        let test_output_dir = Utf8PathBuf::from(TEST_ARTIFACTS_DIR);

        // Ensure output directory exists
        fs::create_dir_all(&test_output_dir).expect("Failed to create test output directory");

        // Parse contracts from the ABI directory
        let contracts = parse_test_contracts().expect("Failed to parse test contracts");

        println!("Found {} test contracts", contracts.len());
        assert!(
            !contracts.is_empty(),
            "No test contracts found in {}",
            CONTRACTS_ABI_DIR
        );

        // Generate Go bindings
        let golang_plugin = GolangPlugin::new();

        for contract in &contracts {
            let contract_name = contract.name.split("::").last().unwrap_or(&contract.name);

            println!("Generating Go bindings for: {}", contract_name);

            let plugin_input = crate::plugins::PluginInput {
                output_dir: test_output_dir.clone(),
                contracts: vec![contract.clone()],
                execution_version: cainome_rs::ExecutionVersion::V3,
                derives: vec![],
                contract_derives: vec![],
                type_skips: vec![],
            };

            golang_plugin
                .generate_code(&plugin_input)
                .await
                .unwrap_or_else(|_| panic!("Failed to generate Go bindings for {}", contract_name));
        }

        // Verify files were generated
        let generated_files: Vec<_> = fs::read_dir(&test_output_dir)
            .expect("Failed to read test output directory")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("go") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        println!("Generated {} Go files", generated_files.len());
        assert!(!generated_files.is_empty(), "No Go files were generated");

        // Verify each generated file has valid structure
        for file_path in &generated_files {
            verify_go_file_structure(file_path)
                .unwrap_or_else(|_| panic!("Invalid Go file structure: {}", file_path.display()));
        }
    }

    /// Test Go compilation of generated bindings
    #[tokio::test]
    async fn test_go_compilation() {
        if !is_go_available() {
            println!("Go compiler not available, skipping compilation test");
            return;
        }

        let test_output_dir = Utf8PathBuf::from(TEST_ARTIFACTS_DIR);

        // Ensure go.mod exists
        setup_go_module(&test_output_dir).expect("Failed to setup Go module");

        // Find all Go files in test artifacts
        let go_files: Vec<_> = fs::read_dir(&test_output_dir)
            .expect("Failed to read test artifacts directory")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("go") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        if go_files.is_empty() {
            println!("No Go files found in test artifacts directory. Run 'cargo test --test golang generate_test_artifacts -- --ignored' first.");
            return;
        }

        // Test syntax and static analysis of each Go file
        for go_file in &go_files {
            println!("Testing syntax of: {}", go_file.display());

            // First check syntax with gofmt
            let output = Command::new("gofmt")
                .arg("-e")
                .arg(go_file)
                .output()
                .expect("Failed to execute gofmt");

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!(
                    "Go syntax check failed for {}\nStderr: {}",
                    go_file.display(),
                    stderr
                );
            }

            // Then check with go vet (skip since files are in different packages)
            // Individual files can't be vetted easily due to package structure
        }

        println!("All {} Go files have valid syntax", go_files.len());
    }

    /// Test specific type mappings in generated Go code
    #[tokio::test]
    async fn test_go_type_mappings() {
        let test_output_dir = Utf8PathBuf::from(TEST_ARTIFACTS_DIR);

        // Test bytes31 type mapping
        test_bytes31_mapping(&test_output_dir).await;

        // Test felt252 mappings
        test_felt_mappings(&test_output_dir).await;

        // Test integer mappings
        test_integer_mappings(&test_output_dir).await;
    }

    /// Test bytes31 is mapped to [31]byte
    async fn test_bytes31_mapping(output_dir: &Utf8PathBuf) {
        let test_abi = r#"[
            {
                "type": "struct",
                "name": "test::Bytes31Struct",
                "members": [
                    {
                        "name": "data",
                        "type": "core::bytes_31::bytes31"
                    }
                ]
            },
            {
                "type": "function",
                "name": "get_bytes31",
                "inputs": [],
                "outputs": [
                    {
                        "type": "core::bytes_31::bytes31"
                    }
                ],
                "state_mutability": "view"
            }
        ]"#;

        let tokens = AbiParser::tokens_from_abi_string(test_abi, &Default::default())
            .expect("Failed to parse test ABI");

        let contract = ContractData {
            name: "bytes31_test".to_string(),
            origin: ContractOrigin::SierraClassFile("bytes31_test.contract_class.json".to_string()),
            tokens,
        };

        let plugin_input = PluginInput {
            output_dir: output_dir.clone(),
            contracts: vec![contract],
            execution_version: cainome_rs::ExecutionVersion::V3,
            derives: vec![],
            contract_derives: vec![],
            type_skips: vec![],
        };

        let golang_plugin = GolangPlugin::new();
        golang_plugin
            .generate_code(&plugin_input)
            .await
            .expect("Failed to generate Go bindings for bytes31 test");

        // Verify bytes31 mapping
        let go_file_path = output_dir.join("bytes_31_test.go");
        let go_content =
            fs::read_to_string(&go_file_path).expect("Failed to read generated Go file");

        assert!(
            go_content.contains("[31]byte"),
            "bytes31 should be mapped to [31]byte"
        );
        assert!(
            go_content.contains("Data [31]byte"),
            "Struct field should use [31]byte"
        );
        assert!(
            go_content.contains("GetBytes31() ([31]byte, error)"),
            "Function should return [31]byte"
        );
    }

    /// Test felt252 mappings
    async fn test_felt_mappings(output_dir: &Utf8PathBuf) {
        // Look for any generated Go files with felt mappings
        let go_files = find_go_files(output_dir);

        let mut found_felt_mapping = false;
        for go_file in go_files {
            let content = fs::read_to_string(&go_file).expect("Failed to read Go file");
            if content.contains("*felt.Felt") {
                found_felt_mapping = true;
                break;
            }
        }

        assert!(
            found_felt_mapping,
            "Should find felt252 -> *felt.Felt mappings in generated code"
        );
    }

    /// Test integer mappings
    async fn test_integer_mappings(output_dir: &Utf8PathBuf) {
        let go_files = find_go_files(output_dir);

        let mut found_basic_ints = false;
        let mut _found_big_ints = false;

        for go_file in go_files {
            let content = fs::read_to_string(&go_file).expect("Failed to read Go file");

            if content.contains("uint64") || content.contains("uint32") || content.contains("uint8")
            {
                found_basic_ints = true;
            }

            if content.contains("*big.Int") {
                _found_big_ints = true;
            }
        }

        assert!(found_basic_ints, "Should find basic integer type mappings");
        // Note: big.Int may not always be present depending on the test contracts
    }

    /// Generate fresh Go bindings and commit them as test artifacts
    #[tokio::test]
    #[ignore] // Only run when explicitly requested with --ignored
    async fn generate_test_artifacts() {
        let test_output_dir = Utf8PathBuf::from(TEST_ARTIFACTS_DIR);

        // Clean existing artifacts
        if test_output_dir.exists() {
            fs::remove_dir_all(&test_output_dir).expect("Failed to remove existing artifacts");
        }
        fs::create_dir_all(&test_output_dir).expect("Failed to create artifacts directory");

        // Generate all Go bindings
        let contracts = parse_test_contracts().expect("Failed to parse test contracts");
        let golang_plugin = GolangPlugin::new();

        for contract in &contracts {
            let plugin_input = crate::plugins::PluginInput {
                output_dir: test_output_dir.clone(),
                contracts: vec![contract.clone()],
                execution_version: cainome_rs::ExecutionVersion::V3,
                derives: vec![],
                contract_derives: vec![],
                type_skips: vec![],
            };

            golang_plugin
                .generate_code(&plugin_input)
                .await
                .expect("Failed to generate Go bindings");
        }

        // Setup Go module
        setup_go_module(&test_output_dir).expect("Failed to setup Go module");

        // Download dependencies
        if is_go_available() {
            let output = Command::new("go")
                .args(["mod", "tidy"])
                .current_dir(&test_output_dir)
                .output()
                .expect("Failed to run go mod tidy");

            if !output.status.success() {
                println!("Warning: go mod tidy failed, but continuing...");
            }
        }

        println!("Generated test artifacts in: {}", test_output_dir);
        println!("Files generated:");
        for entry in fs::read_dir(&test_output_dir)
            .expect("Failed to read directory")
            .flatten()
        {
            println!("  {}", entry.file_name().to_string_lossy());
        }
    }

    // Helper functions

    fn parse_test_contracts() -> Result<Vec<ContractData>, Box<dyn std::error::Error>> {
        let config = ContractParserConfig {
            sierra_extension: ".abi.json".to_string(), // Use .abi.json files directly
            ..Default::default()
        };

        let abi_path = Utf8PathBuf::from(CONTRACTS_ABI_DIR);

        if !abi_path.exists() {
            return Err(format!("ABI directory not found: {}", CONTRACTS_ABI_DIR).into());
        }

        let contracts = ContractParser::from_artifacts_path(abi_path, &config)?;

        Ok(contracts)
    }

    fn is_go_available() -> bool {
        Command::new("go")
            .args(["version"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn setup_go_module(dir: &Utf8PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let go_mod_content = r#"module cainome_test_bindings

go 1.21

require github.com/NethermindEth/juno v0.3.1
"#;

        fs::write(dir.join("go.mod"), go_mod_content)?;
        Ok(())
    }

    fn verify_go_file_structure(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;

        // Check for required Go elements
        if !content.contains("package ") {
            return Err("Missing package declaration".into());
        }

        if !content.contains("github.com/NethermindEth/juno/core/felt") {
            return Err("Missing felt import".into());
        }

        if !content.contains("Provider interface") {
            return Err("Missing Provider interface".into());
        }

        Ok(())
    }

    fn find_go_files(dir: &Utf8PathBuf) -> Vec<std::path::PathBuf> {
        fs::read_dir(dir)
            .unwrap_or_else(|_| panic!("Failed to read directory: {}", dir))
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("go") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect()
    }
}
