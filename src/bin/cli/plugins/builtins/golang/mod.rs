use async_trait::async_trait;
use convert_case::{Case, Casing};
use std::collections::HashMap;

use cainome_parser::tokens::{
    Composite, CompositeInnerKind, CompositeType, Function, StateMutability, Token,
};

#[cfg(test)]
use cainome_rs;

use crate::args::GolangPluginOptions;
use crate::error::CainomeCliResult;
use crate::plugins::builtins::BuiltinPlugin;
use crate::plugins::PluginInput;

pub struct GolangPlugin {
    options: GolangPluginOptions,
}

impl GolangPlugin {
    pub fn new(options: GolangPluginOptions) -> Self {
        Self { options }
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
    fn token_to_go_type(&self, token: &Token, contract_name: &str) -> String {
        match token {
            Token::CoreBasic(core_basic) => self.map_core_basic_type(&core_basic.type_path),
            Token::Array(array) => {
                let inner_type = self.token_to_go_type(&array.inner, contract_name);
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
                        .map(|(i, token)| {
                            format!("Field{} {}", i, self.token_to_go_type(token, contract_name))
                        })
                        .collect();
                    format!("struct {{\n\t{}\n}}", field_types.join("\n\t"))
                }
            }
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    self.map_composite_builtin_type(&composite.type_path_no_generic())
                } else {
                    // Use actual type name without contract prefix
                    composite.type_name_or_alias().to_case(Case::Pascal)
                }
            }
            Token::Option(option) => {
                let inner_type = self.token_to_go_type(&option.inner, contract_name);
                format!("*{}", inner_type) // Use pointer for optional types
            }
            Token::Result(result) => {
                // Generate a Result type that can be unpacked into (value, error) pattern
                let ok_type = self.token_to_go_type(&result.inner, contract_name);
                let err_type = self.token_to_go_type(&result.error, contract_name);
                format!("Result[{}, {}]", ok_type, err_type)
            }
            Token::NonZero(non_zero) => self.token_to_go_type(&non_zero.inner, contract_name),
            Token::Function(_) => "func".to_string(),
        }
    }

    /// Generates Go struct definition for a Cairo composite type
    fn generate_struct(&self, composite: &Composite, contract_name: &str) -> String {
        let struct_name = composite.type_name_or_alias().to_case(Case::Pascal);
        let mut struct_def = format!("type {} struct {{\n", struct_name);

        for inner in &composite.inners {
            let field_name = inner.name.to_case(Case::Pascal);
            let field_type = self.token_to_go_type(&inner.token, contract_name);
            let json_tag = format!("`json:\"{}\"`", inner.name);
            struct_def.push_str(&format!("\t{} {} {}\n", field_name, field_type, json_tag));
        }

        struct_def.push_str("}\n");

        // Check if this is an event struct and generate interface implementation
        if self.is_event_struct(&struct_name) {
            struct_def.push_str(&self.generate_event_interface_methods(&struct_name));
        }

        struct_def
    }

    /// Checks if a struct name represents an event
    fn is_event_struct(&self, struct_name: &str) -> bool {
        // Event structs typically start with "Event" or contain event-like naming
        struct_name.starts_with("Event") || 
        struct_name.starts_with("My") && struct_name.contains("Event") ||
        // Add other patterns as needed
        struct_name.ends_with("Event") && !struct_name.ends_with("sEvent") // Avoid matching things like "SimpleEventsEvent"
    }

    /// Generates interface methods for event structs  
    fn generate_event_interface_methods(&self, struct_name: &str) -> String {
        // Skip generating interface methods here - we'll handle this differently
        // Event structs need to implement the interface from the event enum, not individual methods

        let mut methods = String::new();
        methods.push('\n');

        // Generate EventName method for identification
        methods.push_str(&format!(
            "// EventName returns the name of this event type\n"
        ));
        methods.push_str(&format!(
            "func (e {}) EventName() string {{\n\treturn \"{}\"\n}}\n\n",
            struct_name,
            struct_name.replace("Event", "").to_case(Case::Snake)
        ));

        methods
    }

    /// Generates Go enum definition for a Cairo enum type
    fn generate_enum(&self, composite: &Composite, contract_name: &str) -> String {
        let enum_name = composite.type_name_or_alias().to_case(Case::Pascal);

        // Check if this is an event enum (ends with "Event")
        if enum_name.ends_with("Event") {
            return self.generate_event_enum(composite, contract_name);
        }

        let mut enum_def = String::new();

        // Generate the enum type
        enum_def.push_str(&format!("type {} struct {{\n", enum_name));
        enum_def.push_str("\tVariant string `json:\"variant\"`\n");
        enum_def.push_str("\tValue   interface{} `json:\"value,omitempty\"`\n");
        enum_def.push_str("}\n\n");

        // Generate constants for each variant
        if !composite.inners.is_empty() {
            enum_def.push_str("const (\n");
            for inner in &composite.inners {
                let variant_name = format!("{}_{}", enum_name, inner.name.to_case(Case::Pascal));
                enum_def.push_str(&format!("\t{} = \"{}\"\n", variant_name, inner.name));
            }
            enum_def.push_str(")\n\n");
        }

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
                    let data_type = self.token_to_go_type(&inner.token, contract_name);
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

    /// Generates Go event interface for Cairo event enum types (idiomatic Go approach)
    fn generate_event_enum(&self, composite: &Composite, _contract_name: &str) -> String {
        let enum_name = composite.type_name_or_alias().to_case(Case::Pascal);
        let interface_name = enum_name.clone();
        let mut event_def = String::new();

        // Generate the event interface with a single marker method
        event_def.push_str(&format!(
            "// {} represents a contract event\n",
            interface_name
        ));
        event_def.push_str(&format!("type {} interface {{\n", interface_name));

        // Single marker method for the interface
        event_def.push_str(&format!("\tIs{}() bool\n", interface_name));

        event_def.push_str("}\n\n");

        // Generate constants for event names
        if !composite.inners.is_empty() {
            event_def.push_str("const (\n");
            for inner in &composite.inners {
                let variant_name = inner.name.to_case(Case::Pascal);
                let const_name = format!("{}_{}", interface_name, variant_name);
                event_def.push_str(&format!("\t{} = \"{}\"\n", const_name, inner.name));
            }
            event_def.push_str(")\n\n");
        }

        event_def
    }

    /// Generate implementation methods for an event struct to implement the event interface
    fn generate_event_struct_implementation(
        &self,
        struct_name: &str,
        event_enum: &Composite,
        _contract_name: &str,
    ) -> String {
        let interface_name = event_enum.type_name_or_alias().to_case(Case::Pascal);
        let mut impl_methods = String::new();

        // Generate the single marker method implementation
        impl_methods.push_str(&format!(
            "// Is{} implements the {} interface\n",
            interface_name, interface_name
        ));
        impl_methods.push_str(&format!(
            "func (e {}) Is{}() bool {{\n\treturn true\n}}\n\n",
            struct_name, interface_name
        ));

        impl_methods
    }

    /// Generates Go function definition for a Cairo contract function
    fn generate_function(&self, function: &Function, contract_name: &str) -> String {
        let func_name = function.name.to_case(Case::Pascal);
        let receiver_name = contract_name.to_case(Case::Snake);

        // Generate parameters
        let mut params = Vec::new();
        for (param_name, param_token) in &function.inputs {
            let go_type = self.token_to_go_type(param_token, contract_name);
            params.push(format!("{} {}", param_name.to_case(Case::Snake), go_type));
        }

        // Generate return types
        let mut returns = Vec::new();
        for (i, output_token) in function.outputs.iter().enumerate() {
            let go_type = self.token_to_go_type(output_token, contract_name);
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
        contract_def.push_str("\tprovider *rpc.Provider\n");
        contract_def.push_str("}\n\n");

        // Generate constructor
        contract_def.push_str(&format!(
            "func New{}(contractAddress *felt.Felt, provider *rpc.Provider) *{} {{\n",
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
    fn generate_package_header(
        &self,
        package_name: &str,
        needs_big_int: bool,
        needs_fmt: bool,
    ) -> String {
        let mut import_lines = vec![
            "\"github.com/NethermindEth/juno/core/felt\"",
            "\"github.com/NethermindEth/starknet.go/rpc\"",
        ];

        if needs_fmt {
            import_lines.insert(0, "\"fmt\"");
        }

        if needs_big_int {
            import_lines.insert(if needs_fmt { 1 } else { 0 }, "\"math/big\"");
        }

        let imports = format!("import (\n\t{}\n)", import_lines.join("\n\t"));

        format!(
            r#"// Code generated by Cainome. DO NOT EDIT.
// Generated from ABI file.

package {}

{}

"#,
            package_name, imports
        )
    }

    /// Generates a shared types file containing common types like Result
    fn generate_shared_types_file(
        &self,
        input: &PluginInput,
        package_name: &str,
    ) -> CainomeCliResult<()> {
        // Generate minimal header with only necessary imports for types file
        let mut types_content = format!(
            r#"// Code generated by Cainome. DO NOT EDIT.
// Generated from ABI file.

package {}

import (
	"fmt"
)

"#,
            package_name
        );

        // Add Result type definition for handling Cairo Result types
        types_content.push_str(
            r#"// Result type for handling Cairo Result types with idiomatic Go error handling
type Result[T, E any] struct {
	IsOk bool
	Ok   T
	Err  E
}

// NewResultOk creates a successful Result
func NewResultOk[T, E any](value T) Result[T, E] {
	return Result[T, E]{IsOk: true, Ok: value}
}

// NewResultErr creates a failed Result
func NewResultErr[T, E any](err E) Result[T, E] {
	return Result[T, E]{IsOk: false, Err: err}
}

// Unwrap returns the success value and error in idiomatic Go pattern
func (r Result[T, E]) Unwrap() (T, error) {
	if r.IsOk {
		return r.Ok, nil
	}
	var zero T
	// If E implements error interface, use it directly
	if err, ok := any(r.Err).(error); ok {
		return zero, err
	}
	// Otherwise, create a generic error
	return zero, fmt.Errorf("result error: %+v", r.Err)
}

"#,
        );

        // Write types file
        let mut types_path = input.output_dir.clone();
        types_path.push("types.go");

        tracing::trace!("Golang writing shared types file {}", types_path);
        std::fs::write(&types_path, types_content)?;

        Ok(())
    }

    /// Checks if the generated code needs fmt import (for Result error handling)
    fn needs_fmt(&self, contracts: &[&crate::contract::ContractData]) -> bool {
        for contract in contracts {
            if self.needs_fmt_for_tokens(&contract.tokens.structs)
                || self.needs_fmt_for_tokens(&contract.tokens.enums)
                || self.needs_fmt_for_tokens(&contract.tokens.functions)
            {
                return true;
            }

            for functions in contract.tokens.interfaces.values() {
                if self.needs_fmt_for_tokens(functions) {
                    return true;
                }
            }
        }
        false
    }

    /// Checks if a list of tokens needs fmt import
    fn needs_fmt_for_tokens(&self, tokens: &[Token]) -> bool {
        for token in tokens {
            if self.token_needs_fmt(token) {
                return true;
            }
        }
        false
    }

    /// Checks if a specific token needs fmt import (has Result types)
    fn token_needs_fmt(&self, token: &Token) -> bool {
        match token {
            Token::Result(_) => true,
            Token::Composite(composite) => composite
                .inners
                .iter()
                .any(|inner| self.token_needs_fmt(&inner.token)),
            Token::Array(array) => self.token_needs_fmt(&array.inner),
            Token::Tuple(tuple) => tuple.inners.iter().any(|token| self.token_needs_fmt(token)),
            Token::Option(option) => self.token_needs_fmt(&option.inner),
            Token::NonZero(non_zero) => self.token_needs_fmt(&non_zero.inner),
            Token::Function(function) => {
                function
                    .inputs
                    .iter()
                    .any(|(_, token)| self.token_needs_fmt(token))
                    || function
                        .outputs
                        .iter()
                        .any(|token| self.token_needs_fmt(token))
            }
            _ => false,
        }
    }
}

#[async_trait]
impl BuiltinPlugin for GolangPlugin {
    async fn generate_code(&self, input: &PluginInput) -> CainomeCliResult<()> {
        tracing::trace!("Golang plugin requested");

        let package_name = &self.options.package_name;

        // Check if any contract needs Result types and generate shared types file
        let needs_result_types = input
            .contracts
            .iter()
            .any(|contract| self.needs_fmt(&[contract]));
        if needs_result_types {
            self.generate_shared_types_file(input, package_name)?;
        }

        for contract in &input.contracts {
            let contract_name = contract
                .name
                .split("::")
                .last()
                .unwrap_or(&contract.name)
                .from_case(Case::Snake)
                .to_case(Case::Pascal);

            // We'll check if math/big is actually needed by examining the generated code

            // Generate code first to check actual usage
            let mut generated_code_temp = String::new();

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

            // Find event enums first
            let mut event_enums: Vec<&Composite> = Vec::new();
            for composite in composites.values() {
                if composite.r#type == CompositeType::Enum {
                    let enum_name = composite.type_name_or_alias().to_case(Case::Pascal);
                    if enum_name.ends_with("Event") {
                        event_enums.push(composite);
                    }
                }
            }

            // Generate composite types (structs and enums) with contract namespacing
            for composite in composites.values() {
                match composite.r#type {
                    CompositeType::Struct => {
                        // Check if this is an event struct that should implement an event interface
                        let struct_name = composite.type_name_or_alias().to_case(Case::Pascal);
                        let mut struct_code = self.generate_struct(composite, &contract_name);

                        // Find matching event enum and generate interface implementations
                        for event_enum in &event_enums {
                            // Check if this struct is one of the event variants
                            for inner in &event_enum.inners {
                                let variant_name = inner.name.to_case(Case::Pascal);

                                // Match by variant name or by the actual type referenced
                                let matches = if struct_name == variant_name {
                                    true
                                } else {
                                    // Check if the variant's type matches this struct
                                    // Extract the type name from the variant's type path
                                    if let Token::Composite(variant_composite) = &inner.token {
                                        let variant_type_name = variant_composite
                                            .type_name_or_alias()
                                            .to_case(Case::Pascal);
                                        struct_name == variant_type_name
                                    } else {
                                        false
                                    }
                                };

                                if matches {
                                    struct_code.push_str(
                                        &self.generate_event_struct_implementation(
                                            &struct_name,
                                            event_enum,
                                            &contract_name,
                                        ),
                                    );
                                    break;
                                }
                            }
                        }

                        generated_code_temp.push_str(&struct_code);
                        generated_code_temp.push('\n');
                    }
                    CompositeType::Enum => {
                        generated_code_temp
                            .push_str(&self.generate_enum(composite, &contract_name));
                        generated_code_temp.push('\n');
                    }
                    _ => {}
                }
            }

            // Generate contract struct and methods
            generated_code_temp.push_str(&self.generate_contract(&contract_name, &functions));

            // Check if generated code actually uses big.Int
            let needs_big_int = generated_code_temp.contains("*big.Int");

            let mut generated_code =
                self.generate_package_header(package_name, needs_big_int, false);

            // Add the pre-generated content
            generated_code.push_str(&generated_code_temp);

            // Write to file
            let filename = format!("{}.go", contract_name.to_case(Case::Snake));
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

    const TEST_ARTIFACTS_DIR: &str = "src/bin/cli/plugins/builtins/golang/test_artifacts";
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
        let golang_plugin = GolangPlugin::new(crate::args::GolangPluginOptions {
            package_name: "abigen".to_string(),
        });

        for contract in &contracts {
            println!("Generating Go bindings for: {}", contract.name);

            let plugin_input = crate::plugins::PluginInput {
                output_dir: test_output_dir.clone(),
                contracts: vec![contract.clone()],
                execution_version: cainome_rs::ExecutionVersion::V3,
                type_skips: vec![],
            };

            golang_plugin
                .generate_code(&plugin_input)
                .await
                .unwrap_or_else(|_| panic!("Failed to generate Go bindings for {}", contract.name));
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

        // Test Go compilation if Go is available
        if is_go_available() {
            println!("Testing Go compilation of generated bindings");

            // Ensure go.mod exists
            setup_go_module(&test_output_dir).expect("Failed to setup Go module");

            // Test syntax and compilation of each Go file
            for go_file in &generated_files {
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
            }

            // Test that the module can be built
            let build_output = Command::new("go")
                .arg("build")
                .arg("./...")
                .current_dir(&test_output_dir)
                .output()
                .expect("Failed to execute go build");

            if !build_output.status.success() {
                let stderr = String::from_utf8_lossy(&build_output.stderr);
                let stdout = String::from_utf8_lossy(&build_output.stdout);
                panic!("Go build failed\nStdout: {}\nStderr: {}", stdout, stderr);
            }

            println!(
                "All {} Go files have valid syntax and can be built",
                generated_files.len()
            );
        } else {
            println!("Go compiler not available, skipping compilation test");
        }
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
            type_skips: vec![],
        };

        let golang_plugin = GolangPlugin::new(crate::args::GolangPluginOptions {
            package_name: "abigen".to_string(),
        });
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

    // Helper functions

    fn parse_test_contracts() -> Result<Vec<ContractData>, Box<dyn std::error::Error>> {
        let config = ContractParserConfig {
            sierra_extension: ".abi.json".to_string(),
            type_aliases: HashMap::from([
                // Component aliases (existing)
                (
                    "contracts::abicov::components::simple_component::Event".to_string(),
                    "SimpleEvent".to_string(),
                ),
                (
                    "contracts::abicov::components::simple_component::Written".to_string(),
                    "SimpleWritten".to_string(),
                ),
                (
                    "contracts::abicov::components::simple_component::MyStruct".to_string(),
                    "MyStructSimple".to_string(),
                ),
                (
                    "contracts::abicov::components::simple_component_other::Event".to_string(),
                    "OtherEvent".to_string(),
                ),
                (
                    "contracts::abicov::components::simple_component_other::Written".to_string(),
                    "OtherWritten".to_string(),
                ),
                (
                    "contracts::abicov::components::simple_component_other::MyStruct".to_string(),
                    "MyStructOther".to_string(),
                ),
                (
                    "contracts::abicov::components::simple_component::WrittenAB".to_string(),
                    "WrittenAB".to_string(),
                ),
                // Event aliases for all contracts
                (
                    "contracts::abicov::byte_array::byte_array::Event".to_string(),
                    "ByteArrayEvent".to_string(),
                ),
                (
                    "contracts::abicov::option_result::option_result::Event".to_string(),
                    "OptionResultEvent".to_string(),
                ),
                (
                    "contracts::simple_get_set::simple_get_set::Event".to_string(),
                    "SimpleGetSetEvent".to_string(),
                ),
                (
                    "contracts::abicov::simple_types::simple_types::Event".to_string(),
                    "SimpleTypesEvent".to_string(),
                ),
                (
                    "contracts::abicov::enums::enums::Event".to_string(),
                    "EnumsEvent".to_string(),
                ),
                (
                    "contracts::basic::basic::Event".to_string(),
                    "BasicEvent".to_string(),
                ),
                (
                    "contracts::abicov::simple_interface::simple_interface::Event".to_string(),
                    "SimpleInterfaceEvent".to_string(),
                ),
                (
                    "contracts::abicov::simple_events::simple_events::Event".to_string(),
                    "SimpleEventsEvent".to_string(),
                ),
                (
                    "contracts::gen::gen::Event".to_string(),
                    "GenEvent".to_string(),
                ),
                (
                    "contracts::abicov::builtins::builtins::Event".to_string(),
                    "BuiltinsEvent".to_string(),
                ),
                (
                    "contracts::abicov::structs::structs::Event".to_string(),
                    "StructsEvent".to_string(),
                ),
                (
                    "contracts::event::event::Event".to_string(),
                    "EventEvent".to_string(),
                ),
                (
                    "contracts::abicov::components::components_contract::Event".to_string(),
                    "ComponentsContractEvent".to_string(),
                ),
                // MyStruct aliases
                (
                    "contracts::abicov::builtins::builtins::MyStruct".to_string(),
                    "MyStructBuiltins".to_string(),
                ),
                (
                    "contracts::gen::gen::MyStruct".to_string(),
                    "MyStructGen".to_string(),
                ),
                // GenericOne aliases
                (
                    "contracts::abicov::option_result::option_result::GenericOne".to_string(),
                    "GenericOneOptionResult".to_string(),
                ),
                (
                    "contracts::abicov::structs::structs::GenericOne".to_string(),
                    "GenericOneStructs".to_string(),
                ),
            ]),
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
        let go_mod_content = r#"module abigen

go 1.21

require (
    github.com/NethermindEth/juno v0.14.6
    github.com/NethermindEth/starknet.go v0.12.0
)
"#;

        fs::write(dir.join("go.mod"), go_mod_content)?;

        // Run go mod tidy to clean up dependencies
        let tidy_output = Command::new("go")
            .args(["mod", "tidy"])
            .current_dir(dir)
            .output()?;

        if !tidy_output.status.success() {
            let stderr = String::from_utf8_lossy(&tidy_output.stderr);
            return Err(format!("go mod tidy failed: {}", stderr).into());
        }

        Ok(())
    }

    fn verify_go_file_structure(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;

        // Check for required Go elements
        if !content.contains("package ") {
            return Err("Missing package declaration".into());
        }

        // Special case for types.go - it only needs fmt import
        if file_path.file_name().and_then(|s| s.to_str()) == Some("types.go") {
            if !content.contains("\"fmt\"") {
                return Err("Missing fmt import in types.go".into());
            }
            return Ok(());
        }

        // For all other files, check for standard imports
        if !content.contains("github.com/NethermindEth/juno/core/felt") {
            return Err("Missing felt import".into());
        }

        if !content.contains("github.com/NethermindEth/starknet.go/rpc") {
            return Err("Missing starknet rpc import".into());
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
