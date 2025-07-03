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

    /// Sanitizes a contract name to be a valid Go identifier
    fn sanitize_go_identifier(&self, name: &str) -> String {
        // Replace common special characters with underscores
        let mut sanitized = name
            .replace('.', "_")
            .replace('-', "_")
            .replace('/', "_")
            .replace('@', "_at_")
            .replace('+', "_plus_")
            .replace(' ', "_");

        // Remove any remaining invalid characters
        sanitized = sanitized
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        // Ensure it starts with a letter or underscore
        if sanitized.chars().next().map_or(true, |c| c.is_numeric()) {
            sanitized = format!("_{}", sanitized);
        }

        // Remove consecutive underscores
        let mut result = String::new();
        let mut prev_underscore = false;
        for c in sanitized.chars() {
            if c == '_' {
                if !prev_underscore {
                    result.push(c);
                }
                prev_underscore = true;
            } else {
                result.push(c);
                prev_underscore = false;
            }
        }

        // Remove trailing underscores
        result.trim_end_matches('_').to_string()
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

    /// Converts a token to its Go type representation for struct fields
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
                    // Use actual type name without contract prefix for struct fields
                    composite.type_name_or_alias().to_case(Case::Pascal)
                }
            }
            Token::Option(option) => {
                // For all Option types, add a pointer for nullability
                let inner_type = self.token_to_go_type(&option.inner);
                format!("*{}", inner_type)
            }
            Token::Result(result) => {
                // Generate a Result type that can be unpacked into (value, error) pattern
                let ok_type = self.token_to_go_type(&result.inner);
                let err_type = self.token_to_go_type(&result.error);
                format!("cainome.Result[{}, {}]", ok_type, err_type)
            }
            Token::NonZero(non_zero) => self.token_to_go_type(&non_zero.inner),
            Token::Function(_) => "func".to_string(),
        }
    }

    /// Converts a token to its Go type representation for function parameters (with pointers for structs)
    fn token_to_go_param_type(&self, token: &Token) -> String {
        match token {
            Token::CoreBasic(core_basic) => self.map_core_basic_type(&core_basic.type_path),
            Token::Array(array) => {
                let inner_type = self.token_to_go_param_type(&array.inner);
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
                            format!("Field{} {}", i, self.token_to_go_param_type(token))
                        })
                        .collect();
                    format!("struct {{\n\t{}\n}}", field_types.join("\n\t"))
                }
            }
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    self.map_composite_builtin_type(&composite.type_path_no_generic())
                } else if composite.r#type == CompositeType::Enum {
                    // Enum interfaces should not be pointers
                    composite.type_name_or_alias().to_case(Case::Pascal)
                } else {
                    // Use pointer to struct type for pass-by-reference parameters
                    let type_name = composite.type_name_or_alias().to_case(Case::Pascal);
                    format!("*{}", type_name)
                }
            }
            Token::Option(option) => {
                // For all Option types, add a pointer for nullability
                // Use token_to_go_type instead of token_to_go_param_type to avoid double pointers
                let inner_type = self.token_to_go_type(&option.inner);
                format!("*{}", inner_type)
            }
            Token::Result(result) => {
                // Generate a Result type that can be unpacked into (value, error) pattern
                let ok_type = self.token_to_go_param_type(&result.inner);
                let err_type = self.token_to_go_param_type(&result.error);
                format!("cainome.Result[{}, {}]", ok_type, err_type)
            }
            Token::NonZero(non_zero) => self.token_to_go_param_type(&non_zero.inner),
            Token::Function(_) => "func".to_string(),
        }
    }

    /// Generates Go struct definition for a Cairo composite type
    fn generate_struct(&self, composite: &Composite, contract_name: Option<&str>) -> String {
        let mut struct_name = composite.type_name_or_alias().to_case(Case::Pascal);

        // Check if this is an event struct and prefix with contract name if provided
        if self.is_event_struct(&struct_name) {
            if let Some(contract) = contract_name {
                let sanitized_contract = self.sanitize_go_identifier(contract);
                let contract_pascal = sanitized_contract.to_case(Case::Pascal);
                struct_name = format!("{}{}", contract_pascal, struct_name);
            }
        }

        let mut struct_def = format!("type {} struct {{\n", struct_name);

        // Sort fields by name for deterministic output
        let mut indexed_inners: Vec<(usize, &cainome_parser::tokens::CompositeInner)> =
            composite.inners.iter().enumerate().collect();
        indexed_inners.sort_by(|a, b| a.1.name.cmp(&b.1.name));

        for (_original_index, inner) in &indexed_inners {
            let field_name = inner.name.to_case(Case::Pascal);
            let field_type = self.token_to_go_type(&inner.token);
            let json_tag = format!("`json:\"{}\"`", inner.name);
            struct_def.push_str(&format!("\t{} {} {}\n", field_name, field_type, json_tag));
        }

        struct_def.push_str("}\n\n");

        // Generate CairoMarshaler implementation for the struct
        struct_def.push_str(&self.generate_struct_cairo_marshaler(&struct_name, composite));

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
        methods.push_str("// EventName returns the name of this event type\n");
        methods.push_str(&format!(
            "func (e {}) EventName() string {{\n\treturn \"{}\"\n}}\n\n",
            struct_name,
            struct_name.replace("Event", "").to_case(Case::Snake)
        ));

        methods
    }

    /// Generates Go enum definition for a Cairo enum type
    fn generate_enum(&self, composite: &Composite, contract_name: Option<&str>) -> String {
        let mut enum_name = composite.type_name_or_alias().to_case(Case::Pascal);

        // Check if this is an event enum (ends with "Event") and prefix with contract name if provided
        if enum_name.ends_with("Event") {
            if let Some(contract) = contract_name {
                let sanitized_contract = self.sanitize_go_identifier(contract);
                let contract_pascal = sanitized_contract.to_case(Case::Pascal);
                enum_name = format!("{}{}", contract_pascal, enum_name);
            }
            return self.generate_event_enum_with_name(composite, &enum_name);
        }

        let mut enum_def = String::new();

        // Generate the enum interface (like events)
        enum_def.push_str(&format!("// {} represents a Cairo enum type\n", enum_name));
        enum_def.push_str(&format!("type {} interface {{\n", enum_name));
        enum_def.push_str(&format!("\tIs{}() bool\n", enum_name));
        enum_def.push_str("\tMarshalCairo() ([]*felt.Felt, error)\n");
        enum_def.push_str("\tUnmarshalCairo(data []*felt.Felt) error\n");
        enum_def.push_str("}\n\n");

        // Suppress unused parameter warning for non-event enums
        let _ = contract_name;

        // Generate constants for each variant
        if !composite.inners.is_empty() {
            enum_def.push_str("const (\n");
            // Sort variants by name for deterministic output
            let mut indexed_inners: Vec<(usize, &cainome_parser::tokens::CompositeInner)> =
                composite.inners.iter().enumerate().collect();
            indexed_inners.sort_by(|a, b| a.1.name.cmp(&b.1.name));

            for (_original_index, inner) in &indexed_inners {
                let variant_name = format!("{}_{}", enum_name, inner.name.to_case(Case::Pascal));
                enum_def.push_str(&format!("\t{} = \"{}\"\n", variant_name, inner.name));
            }
            enum_def.push_str(")\n\n");
        }

        // Generate individual variant types that implement the interface
        // Sort variants by name for deterministic output but preserve original indices
        let mut indexed_inners: Vec<(usize, &cainome_parser::tokens::CompositeInner)> =
            composite.inners.iter().enumerate().collect();
        indexed_inners.sort_by(|a, b| a.1.name.cmp(&b.1.name));

        for (_original_index, inner) in &indexed_inners {
            let variant_name = inner.name.to_case(Case::Pascal);
            let variant_type_name = format!("{}{}", enum_name, variant_name);

            match inner.kind {
                CompositeInnerKind::NotUsed => {
                    // Unit variant (no data) - empty struct
                    enum_def.push_str(&format!("type {} struct {{}}\n\n", variant_type_name));

                    // Constructor function
                    enum_def.push_str(&format!(
                        "func New{}() {} {{\n\treturn {}{{}}\n}}\n\n",
                        variant_type_name, variant_type_name, variant_type_name
                    ));
                }
                CompositeInnerKind::Data => {
                    // Data variant - struct with data field
                    let data_type = self.token_to_go_type(&inner.token);
                    enum_def.push_str(&format!(
                        "type {} struct {{\n\tData {} `json:\"data\"`\n}}\n\n",
                        variant_type_name, data_type
                    ));

                    // Constructor function
                    enum_def.push_str(&format!(
                        "func New{}(data {}) {} {{\n\treturn {} {{Data: data}}\n}}\n\n",
                        variant_type_name, data_type, variant_type_name, variant_type_name
                    ));
                }
                _ => {}
            }

            // Implement the interface for this variant
            enum_def.push_str(&format!(
                "// Is{} implements the {} interface\n",
                enum_name, enum_name
            ));
            enum_def.push_str(&format!(
                "func ({} {}) Is{}() bool {{\n\treturn true\n}}\n\n",
                variant_name.to_lowercase().chars().next().unwrap_or('v'),
                variant_type_name,
                enum_name
            ));

            // Generate CairoMarshaler implementation for each variant
            enum_def.push_str(&self.generate_enum_variant_cairo_marshaler(
                &variant_type_name,
                &inner.token,
                inner.kind,
                composite,
                inner.index as u64,
            ));
        }

        enum_def
    }

    /// Generate CairoMarshaler implementation for an enum variant
    fn generate_enum_variant_cairo_marshaler(
        &self,
        variant_type_name: &str,
        token: &Token,
        kind: CompositeInnerKind,
        _enum_composite: &Composite,
        discriminant: u64,
    ) -> String {
        let mut marshaler = String::new();

        // MarshalCairo implementation
        marshaler.push_str(&format!(
            "// MarshalCairo serializes {} to Cairo felt array\n",
            variant_type_name
        ));
        marshaler.push_str(&format!(
            "func ({} *{}) MarshalCairo() ([]*felt.Felt, error) {{\n",
            variant_type_name
                .to_lowercase()
                .chars()
                .next()
                .unwrap_or('v'),
            variant_type_name
        ));
        marshaler.push_str("\tvar result []*felt.Felt\n\n");
        marshaler.push_str("\t// Discriminant for variant\n");
        marshaler.push_str(&format!(
            "\tresult = append(result, cainome.FeltFromUint({}))\n",
            discriminant
        ));

        match kind {
            CompositeInnerKind::NotUsed => {
                marshaler.push_str("\t// Unit variant - no additional data\n");
            }
            CompositeInnerKind::Data => {
                // Add data serialization
                let receiver_var = variant_type_name
                    .to_lowercase()
                    .chars()
                    .next()
                    .unwrap_or('v');
                marshaler.push_str(
                    &self.generate_enum_variant_marshal_code_new(token, &receiver_var.to_string()),
                );
            }
            _ => {}
        }

        marshaler.push_str("\n\treturn result, nil\n");
        marshaler.push_str("}\n\n");

        // UnmarshalCairo implementation
        marshaler.push_str(&format!(
            "// UnmarshalCairo deserializes {} from Cairo felt array\n",
            variant_type_name
        ));
        marshaler.push_str(&format!(
            "func ({} *{}) UnmarshalCairo(data []*felt.Felt) error {{\n",
            variant_type_name
                .to_lowercase()
                .chars()
                .next()
                .unwrap_or('v'),
            variant_type_name
        ));
        marshaler.push_str("\tif len(data) == 0 {\n");
        marshaler.push_str("\t\treturn fmt.Errorf(\"insufficient data for enum discriminant\")\n");
        marshaler.push_str("\t}\n\n");
        marshaler.push_str("\tdiscriminant := cainome.UintFromFelt(data[0])\n");
        marshaler.push_str(&format!("\tif discriminant != {} {{\n", discriminant));
        marshaler.push_str(&format!(
            "\t\treturn fmt.Errorf(\"expected discriminant {}, got %d\", discriminant)\n",
            discriminant
        ));
        marshaler.push_str("\t}\n");
        marshaler.push_str("\toffset := 1\n\n");

        match kind {
            CompositeInnerKind::NotUsed => {
                marshaler.push_str("\t// Unit variant - no additional data to unmarshal\n");
                marshaler.push_str("\t_ = offset // Suppress unused variable warning\n");
            }
            CompositeInnerKind::Data => {
                // Add data deserialization
                let receiver_var = variant_type_name
                    .to_lowercase()
                    .chars()
                    .next()
                    .unwrap_or('v');
                marshaler.push_str(
                    &self
                        .generate_enum_variant_unmarshal_code_new(token, &receiver_var.to_string()),
                );
            }
            _ => {}
        }

        marshaler.push_str("\treturn nil\n");
        marshaler.push_str("}\n\n");

        // CairoSize implementation
        marshaler.push_str(&format!(
            "// CairoSize returns the serialized size for {}\n",
            variant_type_name
        ));
        marshaler.push_str(&format!(
            "func ({} *{}) CairoSize() int {{\n",
            variant_type_name
                .to_lowercase()
                .chars()
                .next()
                .unwrap_or('v'),
            variant_type_name
        ));
        marshaler.push_str("\treturn -1 // Dynamic size\n");
        marshaler.push_str("}\n\n");

        marshaler
    }

    /// Generate enum interface deserialization logic
    fn generate_enum_interface_deserialization(
        &self,
        _return_type: &str,
        enum_name: &str,
        token: &Token,
    ) -> String {
        let mut code = String::new();

        if let Token::Composite(composite) = token {
            code.push_str("\tif len(response) == 0 {\n");
            code.push_str("\t\treturn nil, fmt.Errorf(\"empty response\")\n");
            code.push_str("\t}\n");
            code.push_str("\t\n");
            code.push_str("\t// Read discriminant to determine variant\n");
            code.push_str("\tdiscriminant := cainome.UintFromFelt(response[0])\n");
            code.push_str("\t\n");
            code.push_str("\tswitch discriminant {\n");

            // Generate cases for each variant (preserve original indices for discriminants)
            let mut indexed_inners: Vec<(usize, &cainome_parser::tokens::CompositeInner)> =
                composite.inners.iter().enumerate().collect();
            // Sort by name for deterministic output but preserve original index
            indexed_inners.sort_by(|a, b| a.1.name.cmp(&b.1.name));

            for (original_index, inner) in indexed_inners {
                let variant_name = inner.name.to_case(Case::Pascal);
                let variant_type_name = format!("{}{}", enum_name, variant_name);

                code.push_str(&format!("\tcase {}:\n", original_index));
                code.push_str(&format!("\t\tvar result {}\n", variant_type_name));
                code.push_str("\t\tif err := result.UnmarshalCairo(response); err != nil {\n");
                code.push_str(
                    "\t\t\treturn nil, fmt.Errorf(\"failed to unmarshal variant: %w\", err)\n",
                );
                code.push_str("\t\t}\n");
                code.push_str("\t\treturn &result, nil\n");
            }

            code.push_str("\tdefault:\n");
            code.push_str(
                "\t\treturn nil, fmt.Errorf(\"unknown enum discriminant: %d\", discriminant)\n",
            );
            code.push_str("\t}\n");
        } else {
            code.push_str("\t// TODO: Handle non-composite enum type\n");
            code.push_str("\treturn nil, fmt.Errorf(\"unsupported enum type\")\n");
        }

        code
    }

    /// Generate marshal code for enum variant with new interface approach
    fn generate_enum_variant_marshal_code_new(&self, token: &Token, receiver_var: &str) -> String {
        match token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    format!("\tresult = append(result, {}.Data)\n", receiver_var)
                }
                "core::bool" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromBool({}.Data))\n",
                        receiver_var
                    )
                }
                "core::integer::u8"
                | "core::integer::u16"
                | "core::integer::u32"
                | "core::integer::u64"
                | "core::integer::usize" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromUint(uint64({}.Data)))\n",
                        receiver_var
                    )
                }
                "core::integer::u128" | "core::integer::i128" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromBigInt({}.Data))\n",
                        receiver_var
                    )
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                | "core::integer::i64" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromInt(int64({}.Data)))\n",
                        receiver_var
                    )
                }
                "core::starknet::contract_address::ContractAddress"
                | "core::starknet::class_hash::ClassHash" => {
                    format!("\tresult = append(result, {}.Data)\n", receiver_var)
                }
                _ => format!(
                    "\t// TODO: Handle unknown core basic type {} for enum variant\n",
                    core_basic.type_path
                ),
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            format!(
                                "\tresult = append(result, cainome.FeltFromBigInt({}.Data))\n",
                                receiver_var
                            )
                        }
                        "core::starknet::eth_address::EthAddress" => {
                            format!(
                                "\tresult = append(result, cainome.FeltFromBytes({}.Data[:]))\n",
                                receiver_var
                            )
                        }
                        "core::byte_array::ByteArray" => {
                            format!(
                                "\tif byteArrayData, err := cainome.NewCairoByteArray({}.Data).MarshalCairo(); err != nil {{\n\t\treturn nil, fmt.Errorf(\"failed to marshal ByteArray enum variant: %w\", err)\n\t}} else {{\n\t\tresult = append(result, byteArrayData...)\n\t}}\n",
                                receiver_var
                            )
                        }
                        _ => "\t// TODO: Handle unknown builtin composite type for enum variant\n"
                            .to_string(),
                    }
                } else {
                    format!("\tif valueData, err := {}.Data.MarshalCairo(); err != nil {{\n\t\treturn nil, err\n\t}} else {{\n\t\tresult = append(result, valueData...)\n\t}}\n", receiver_var)
                }
            }
            Token::Tuple(tuple) => {
                // Handle tuple variant data marshalling
                let mut marshal_code = String::new();
                for (i, inner_token) in tuple.inners.iter().enumerate() {
                    match inner_token {
                        Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                            "felt" | "core::felt252" => {
                                marshal_code.push_str(&format!(
                                    "\tresult = append(result, {}.Data.Field{})\n",
                                    receiver_var, i
                                ));
                            }
                            "core::integer::u8"
                            | "core::integer::u16"
                            | "core::integer::u32"
                            | "core::integer::u64"
                            | "core::integer::usize" => {
                                marshal_code.push_str(&format!("\tresult = append(result, cainome.FeltFromUint(uint64({}.Data.Field{})))\n", receiver_var, i));
                            }
                            "core::integer::u128" | "core::integer::i128" => {
                                marshal_code.push_str(&format!("\tresult = append(result, cainome.FeltFromBigInt({}.Data.Field{}))\n", receiver_var, i));
                            }
                            "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                            | "core::integer::i64" => {
                                marshal_code.push_str(&format!("\tresult = append(result, cainome.FeltFromInt(int64({}.Data.Field{})))\n", receiver_var, i));
                            }
                            _ => {
                                marshal_code.push_str(&format!(
                                    "\t// TODO: Handle unknown core basic type {} in tuple\n",
                                    core_basic.type_path
                                ));
                            }
                        },
                        Token::Composite(composite) => {
                            if composite.is_builtin() {
                                match composite.type_path_no_generic().as_str() {
                                    "core::integer::u256" => {
                                        marshal_code.push_str(&format!("\tresult = append(result, cainome.FeltFromBigInt({}.Data.Field{}))\n", receiver_var, i));
                                    }
                                    "core::starknet::eth_address::EthAddress" => {
                                        marshal_code.push_str(&format!("\tresult = append(result, cainome.FeltFromBytes({}.Data.Field{}[:]))\n", receiver_var, i));
                                    }
                                    "core::byte_array::ByteArray" => {
                                        marshal_code.push_str(&format!("\tif field{}_data, err := cainome.NewCairoByteArray({}.Data.Field{}).MarshalCairo(); err != nil {{\n\t\treturn nil, fmt.Errorf(\"failed to marshal ByteArray tuple field {}: %w\", err)\n\t}} else {{\n\t\tresult = append(result, field{}_data...)\n\t}}\n", i, receiver_var, i, i, i));
                                    }
                                    _ => {
                                        marshal_code.push_str(&format!(
                                            "\t// TODO: Handle unknown builtin {} in tuple\n",
                                            composite.type_path
                                        ));
                                    }
                                }
                            } else {
                                marshal_code.push_str(&format!("\tif field{}_data, err := {}.Data.Field{}.MarshalCairo(); err != nil {{\n\t\treturn nil, err\n\t}} else {{\n\t\tresult = append(result, field{}_data...)\n\t}}\n", i, receiver_var, i, i));
                            }
                        }
                        _ => {
                            marshal_code.push_str(&format!(
                                "\t// TODO: Handle unknown token type in tuple field {}\n",
                                i
                            ));
                        }
                    }
                }
                marshal_code
            }
            Token::Array(array) => {
                // Handle array types for enum variants
                match array.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // Array of felt - serialize as length + elements
                        format!("\tresult = append(result, cainome.FeltFromUint(uint64(len({}.Data))))\n\tresult = append(result, {}.Data...)\n", receiver_var, receiver_var)
                    }
                    _ => {
                        // For other array types, marshal each element
                        format!("\tresult = append(result, cainome.FeltFromUint(uint64(len({}.Data))))\n\tfor _, item := range {}.Data {{\n\t\tif itemData, err := item.MarshalCairo(); err != nil {{\n\t\t\treturn nil, err\n\t\t}} else {{\n\t\t\tresult = append(result, itemData...)\n\t\t}}\n\t}}\n", receiver_var, receiver_var)
                    }
                }
            }
            _ => "\t// TODO: Handle unknown token type for enum variant\n".to_string(),
        }
    }

    /// Generate unmarshal code for enum variant with new interface approach
    fn generate_enum_variant_unmarshal_code_new(
        &self,
        token: &Token,
        receiver_var: &str,
    ) -> String {
        match token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t{}.Data = data[offset]\n\toffset++\n", receiver_var)
                }
                "core::bool" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t{}.Data = cainome.UintFromFelt(data[offset]) != 0\n\toffset++\n", receiver_var)
                }
                "core::integer::u8"
                | "core::integer::u16"
                | "core::integer::u32"
                | "core::integer::u64"
                | "core::integer::usize" => {
                    let go_type = self.token_to_go_type(token);
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t{}.Data = {}(cainome.UintFromFelt(data[offset]))\n\toffset++\n", receiver_var, go_type)
                }
                "core::integer::u128" | "core::integer::i128" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t{}.Data = cainome.BigIntFromFelt(data[offset])\n\toffset++\n", receiver_var)
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                | "core::integer::i64" => {
                    let go_type = self.token_to_go_type(token);
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t{}.Data = {}(cainome.IntFromFelt(data[offset]))\n\toffset++\n", receiver_var, go_type)
                }
                "core::starknet::contract_address::ContractAddress"
                | "core::starknet::class_hash::ClassHash" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t{}.Data = data[offset]\n\toffset++\n", receiver_var)
                }
                _ => {
                    // For unknown types, still consume the data to avoid unused offset
                    format!("\t// TODO: Handle unknown core basic type {} for enum variant unmarshal\n\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t// Skip unknown data\n\t_ = data[offset]\n\toffset++\n", core_basic.type_path)
                }
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\t{}.Data = cainome.BigIntFromFelt(data[offset])\n\toffset++\n", receiver_var)
                        }
                        "core::starknet::eth_address::EthAddress" => {
                            format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for variant data\")\n\t}}\n\tethBytes := data[offset].Bytes()\n\tcopy({}.Data[:], ethBytes[:])\n\toffset++\n", receiver_var)
                        }
                        "core::byte_array::ByteArray" => {
                            format!("\t// ByteArray unmarshaling for enum variant\n\tif byteArrayLength := len(data) - offset; byteArrayLength > 0 {{\n\t\tbyteArray := &cainome.CairoByteArray{{}}\n\t\tif err := byteArray.UnmarshalCairo(data[offset:]); err != nil {{\n\t\t\treturn fmt.Errorf(\"failed to unmarshal ByteArray enum variant: %w\", err)\n\t\t}}\n\t\t{}.Data = byteArray.ToBytes()\n\t\t// TODO: Update offset based on consumed data for ByteArray enum variant\n\t}}\n", receiver_var)
                        }
                        _ => "\t// TODO: Handle unknown builtin composite type for enum variant unmarshal\n".to_string(),
                    }
                } else {
                    format!("\tif err := {}.Data.UnmarshalCairo(data[offset:]); err != nil {{\n\t\treturn err\n\t}}\n\t// TODO: Update offset based on consumed data\n", receiver_var)
                }
            }
            Token::Tuple(tuple) => {
                // Handle tuple variant data unmarshalling
                let mut unmarshal_code = String::new();
                for (i, inner_token) in tuple.inners.iter().enumerate() {
                    unmarshal_code.push_str(&format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for tuple field {}\")\n\t}}\n", i));
                    match inner_token {
                        Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                            "felt" | "core::felt252" => {
                                unmarshal_code.push_str(&format!(
                                    "\t{}.Data.Field{} = data[offset]\n\toffset++\n",
                                    receiver_var, i
                                ));
                            }
                            "core::integer::u8"
                            | "core::integer::u16"
                            | "core::integer::u32"
                            | "core::integer::u64"
                            | "core::integer::usize" => {
                                let go_type = self.token_to_go_type(inner_token);
                                unmarshal_code.push_str(&format!("\t{}.Data.Field{} = {}(cainome.UintFromFelt(data[offset]))\n\toffset++\n", receiver_var, i, go_type));
                            }
                            "core::integer::u128" | "core::integer::i128" => {
                                unmarshal_code.push_str(&format!("\t{}.Data.Field{} = cainome.BigIntFromFelt(data[offset])\n\toffset++\n", receiver_var, i));
                            }
                            "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                            | "core::integer::i64" => {
                                let go_type = self.token_to_go_type(inner_token);
                                unmarshal_code.push_str(&format!("\t{}.Data.Field{} = {}(cainome.IntFromFelt(data[offset]))\n\toffset++\n", receiver_var, i, go_type));
                            }
                            _ => {
                                unmarshal_code.push_str(&format!("\t// TODO: Handle unknown core basic type {} in tuple unmarshal\n", core_basic.type_path));
                            }
                        },
                        Token::Composite(composite) => {
                            if composite.is_builtin() {
                                match composite.type_path_no_generic().as_str() {
                                    "core::integer::u256" => {
                                        unmarshal_code.push_str(&format!("\t{}.Data.Field{} = cainome.BigIntFromFelt(data[offset])\n\toffset++\n", receiver_var, i));
                                    }
                                    "core::starknet::eth_address::EthAddress" => {
                                        unmarshal_code.push_str(&format!("\tethBytes{} := data[offset].Bytes()\n\tcopy({}.Data.Field{}[:], ethBytes{}[:])\n\toffset++\n", i, receiver_var, i, i));
                                    }
                                    "core::byte_array::ByteArray" => {
                                        unmarshal_code.push_str(&format!("\t// ByteArray unmarshaling for tuple field {}\n\tif byteArrayLength{} := len(data) - offset; byteArrayLength{} > 0 {{\n\t\tbyteArray{} := &cainome.CairoByteArray{{}}\n\t\tif err := byteArray{}.UnmarshalCairo(data[offset:]); err != nil {{\n\t\t\treturn fmt.Errorf(\"failed to unmarshal ByteArray tuple field {}: %w\", err)\n\t\t}}\n\t\t{}.Data.Field{} = byteArray{}.ToBytes()\n\t\t// TODO: Update offset based on consumed data for ByteArray tuple field {}\n\t}}\n", i, i, i, i, i, i, receiver_var, i, i, i));
                                    }
                                    _ => {
                                        unmarshal_code.push_str(&format!("\t// TODO: Handle unknown builtin {} in tuple unmarshal\n", composite.type_path));
                                    }
                                }
                            } else {
                                unmarshal_code.push_str(&format!("\tif err := {}.Data.Field{}.UnmarshalCairo(data[offset:]); err != nil {{\n\t\treturn err\n\t}}\n\t// TODO: Update offset based on consumed data for field {}\n", receiver_var, i, i));
                            }
                        }
                        _ => {
                            unmarshal_code.push_str(&format!("\t// TODO: Handle unknown token type in tuple field {} unmarshal\n", i));
                        }
                    }
                }
                unmarshal_code
            }
            Token::Array(array) => {
                // Handle array types for enum variant unmarshal
                match array.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // Array of felt - deserialize length then elements
                        format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for array length\")\n\t}}\n\tarrayLength := cainome.UintFromFelt(data[offset])\n\toffset++\n\tif offset+int(arrayLength) > len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for array elements\")\n\t}}\n\t{}.Data = make([]*felt.Felt, arrayLength)\n\tfor i := uint64(0); i < arrayLength; i++ {{\n\t\t{}.Data[i] = data[offset]\n\t\toffset++\n\t}}\n", receiver_var, receiver_var)
                    }
                    _ => {
                        // For other array types, unmarshal each element
                        format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for array length\")\n\t}}\n\tarrayLength := cainome.UintFromFelt(data[offset])\n\toffset++\n\t{}.Data = make([]interface{{}}, arrayLength)\n\tfor i := uint64(0); i < arrayLength; i++ {{\n\t\t// TODO: Unmarshal complex array element at index i\n\t\t// This requires knowing the exact type of array elements\n\t}}\n", receiver_var)
                    }
                }
            }
            _ => "\t// TODO: Handle unknown token type for enum variant unmarshal\n".to_string(),
        }
    }

    /// Generates Go event interface for Cairo event enum types with a custom name
    fn generate_event_enum_with_name(&self, composite: &Composite, enum_name: &str) -> String {
        let interface_name = enum_name.to_string();
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
            // Sort variants by name for deterministic output
            let mut indexed_inners: Vec<(usize, &cainome_parser::tokens::CompositeInner)> =
                composite.inners.iter().enumerate().collect();
            indexed_inners.sort_by(|a, b| a.1.name.cmp(&b.1.name));

            for (_original_index, inner) in &indexed_inners {
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
        contract_name: &str,
    ) -> String {
        // Generate the interface name with contract prefix
        let mut interface_name = event_enum.type_name_or_alias().to_case(Case::Pascal);
        if interface_name.ends_with("Event") {
            let sanitized_contract = self.sanitize_go_identifier(contract_name);
            let contract_pascal = sanitized_contract.to_case(Case::Pascal);
            interface_name = format!("{}{}", contract_pascal, interface_name);
        }

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

    /// Generates CairoMarshaler implementation for a struct
    fn generate_struct_cairo_marshaler(&self, struct_name: &str, composite: &Composite) -> String {
        let mut marshaler = String::new();

        // Generate MarshalCairo method
        marshaler.push_str(&format!(
            "// MarshalCairo serializes {} to Cairo felt array\n",
            struct_name
        ));
        marshaler.push_str(&format!(
            "func (s *{}) MarshalCairo() ([]*felt.Felt, error) {{\n",
            struct_name
        ));
        marshaler.push_str("\tvar result []*felt.Felt\n\n");

        // Serialize each field (sorted for deterministic output)
        let mut indexed_inners: Vec<(usize, &cainome_parser::tokens::CompositeInner)> =
            composite.inners.iter().enumerate().collect();
        indexed_inners.sort_by(|a, b| a.1.name.cmp(&b.1.name));

        for (_original_index, inner) in &indexed_inners {
            let field_name = inner.name.to_case(Case::Pascal);
            marshaler.push_str(&self.generate_field_marshal_code(&field_name, &inner.token));
        }

        marshaler.push_str("\treturn result, nil\n");
        marshaler.push_str("}\n\n");

        // Generate UnmarshalCairo method
        marshaler.push_str(&format!(
            "// UnmarshalCairo deserializes {} from Cairo felt array\n",
            struct_name
        ));
        marshaler.push_str(&format!(
            "func (s *{}) UnmarshalCairo(data []*felt.Felt) error {{\n",
            struct_name
        ));

        // Only declare offset if we have fields to unmarshal
        if !composite.inners.is_empty() {
            marshaler.push_str("\toffset := 0\n\n");
        }

        // Deserialize each field (sorted for deterministic output)
        let mut indexed_inners: Vec<(usize, &cainome_parser::tokens::CompositeInner)> =
            composite.inners.iter().enumerate().collect();
        indexed_inners.sort_by(|a, b| a.1.name.cmp(&b.1.name));

        for (_original_index, inner) in &indexed_inners {
            let field_name = inner.name.to_case(Case::Pascal);
            marshaler.push_str(&self.generate_field_unmarshal_code(&field_name, &inner.token));
        }

        marshaler.push_str("\treturn nil\n");
        marshaler.push_str("}\n\n");

        // Generate CairoSize method
        marshaler.push_str(&format!(
            "// CairoSize returns the serialized size for {}\n",
            struct_name
        ));
        marshaler.push_str(&format!("func (s *{}) CairoSize() int {{\n", struct_name));
        marshaler.push_str("\treturn -1 // Dynamic size\n");
        marshaler.push_str("}\n\n");

        marshaler
    }

    /// Generates marshal code for array fields
    fn generate_array_marshal_code(&self, field_name: &str, inner_token: &Token) -> String {
        let mut code = format!(
            "\t// Array field {}: serialize length then elements\n",
            field_name
        );
        code.push_str(&format!(
            "\tresult = append(result, cainome.FeltFromUint(uint64(len(s.{}))))\n",
            field_name
        ));
        code.push_str(&format!("\tfor _, item := range s.{} {{\n", field_name));

        match inner_token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    code.push_str("\t\tresult = append(result, item)\n");
                }
                "core::bool" => {
                    code.push_str("\t\tresult = append(result, cainome.FeltFromBool(item))\n");
                }
                "core::integer::u8"
                | "core::integer::u16"
                | "core::integer::u32"
                | "core::integer::u64"
                | "core::integer::usize" => {
                    code.push_str(
                        "\t\tresult = append(result, cainome.FeltFromUint(uint64(item)))\n",
                    );
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str("\t\tresult = append(result, cainome.FeltFromBigInt(item))\n");
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                | "core::integer::i64" => {
                    code.push_str(
                        "\t\tresult = append(result, cainome.FeltFromUint(uint64(item)))\n",
                    );
                }
                "core::starknet::contract_address::ContractAddress"
                | "core::starknet::class_hash::ClassHash" => {
                    code.push_str("\t\tresult = append(result, item)\n");
                }
                _ => {
                    code.push_str("\t\t// TODO: Handle unknown basic type in array\n");
                    code.push_str("\t\t_ = item\n");
                }
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            code.push_str(
                                "\t\tresult = append(result, cainome.FeltFromBigInt(item))\n",
                            );
                        }
                        "core::starknet::eth_address::EthAddress" => {
                            code.push_str(
                                "\t\tresult = append(result, cainome.FeltFromBytes(item[:]))\n",
                            );
                        }
                        "core::byte_array::ByteArray" => {
                            code.push_str(
                                "\t\tif itemData, err := cainome.NewCairoByteArray(item).MarshalCairo(); err != nil {\n\t\t\treturn nil, fmt.Errorf(\"failed to marshal ByteArray array item: %w\", err)\n\t\t} else {\n\t\t\tresult = append(result, itemData...)\n\t\t}\n",
                            );
                        }
                        _ => {
                            code.push_str(
                                "\t\t// TODO: Handle unknown builtin composite type in array\n",
                            );
                            code.push_str("\t\t_ = item\n");
                        }
                    }
                } else {
                    // Custom struct/enum - use CairoMarshaler
                    code.push_str("\t\tif itemData, err := item.MarshalCairo(); err != nil {\n");
                    code.push_str("\t\t\treturn nil, err\n");
                    code.push_str("\t\t} else {\n");
                    code.push_str("\t\t\tresult = append(result, itemData...)\n");
                    code.push_str("\t\t}\n");
                }
            }
            _ => {
                code.push_str("\t\t// TODO: Handle unknown token type in array\n");
                code.push_str("\t\t_ = item\n");
            }
        }

        code.push_str("\t}\n");
        code
    }

    /// Generates unmarshal code for array fields
    fn generate_array_unmarshal_code(&self, field_name: &str, inner_token: &Token) -> String {
        let mut code = format!(
            "\t// Array field {}: read length then elements\n",
            field_name
        );
        code.push_str(&format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for array length of {}\")\n\t}}\n", field_name));
        code.push_str(&format!(
            "\tlength{} := cainome.UintFromFelt(data[offset])\n",
            field_name
        ));
        code.push_str("\toffset++\n");

        // Get the Go type for the array element
        let element_type = self.token_to_go_type(inner_token);
        code.push_str(&format!(
            "\ts.{} = make([]{}, length{})\n",
            field_name, element_type, field_name
        ));
        code.push_str(&format!(
            "\tfor i := uint64(0); i < length{}; i++ {{\n",
            field_name
        ));

        match inner_token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = data[offset]\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::bool" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = cainome.BoolFromFelt(data[offset])\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = uint8(cainome.UintFromFelt(data[offset]))\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = uint16(cainome.UintFromFelt(data[offset]))\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = uint32(cainome.UintFromFelt(data[offset]))\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u64" | "core::integer::usize" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = cainome.UintFromFelt(data[offset])\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = cainome.BigIntFromFelt(data[offset])\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = int8(cainome.UintFromFelt(data[offset]))\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = int16(cainome.UintFromFelt(data[offset]))\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = int32(cainome.UintFromFelt(data[offset]))\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i64" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!(
                        "\t\ts.{}[i] = int64(cainome.UintFromFelt(data[offset]))\n",
                        field_name
                    ));
                    code.push_str("\t\toffset++\n");
                }
                "core::starknet::contract_address::ContractAddress"
                | "core::starknet::class_hash::ClassHash" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = data[offset]\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                _ => {
                    code.push_str("\t\t// TODO: Handle unknown basic type in array unmarshal\n");
                }
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                            code.push_str("\t\t}\n");
                            code.push_str(&format!(
                                "\t\ts.{}[i] = cainome.BigIntFromFelt(data[offset])\n",
                                field_name
                            ));
                            code.push_str("\t\toffset++\n");
                        }
                        "core::starknet::eth_address::EthAddress" => {
                            code.push_str("\t\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                            code.push_str("\t\t}\n");
                            code.push_str(&format!(
                                "\t\tethBytes := data[offset].Bytes()\n\t\tcopy(s.{}[i][:], ethBytes[:])\n",
                                field_name
                            ));
                            code.push_str("\t\toffset++\n");
                        }
                        "core::byte_array::ByteArray" => {
                            code.push_str(&format!(
                                "\t\t// ByteArray unmarshaling for array element\n\t\tif byteArrayLength := len(data) - offset; byteArrayLength > 0 {{\n\t\t\tbyteArray := &cainome.CairoByteArray{{}}\n\t\t\tif err := byteArray.UnmarshalCairo(data[offset:]); err != nil {{\n\t\t\t\treturn fmt.Errorf(\"failed to unmarshal ByteArray array element %d of {}: %w\", i, err)\n\t\t\t}}\n\t\t\ts.{}[i] = byteArray.ToBytes()\n\t\t\t// TODO: Update offset based on consumed data for ByteArray array element\n\t\t}}\n",
                                field_name, field_name
                            ));
                        }
                        _ => {
                            code.push_str("\t\t// TODO: Handle unknown builtin composite type in array unmarshal\n");
                        }
                    }
                } else {
                    // Custom struct/enum - use CairoMarshaler
                    code.push_str(&format!("\t\tvar item {}\n", element_type));
                    code.push_str(
                        "\t\tif err := item.UnmarshalCairo(data[offset:]); err != nil {\n",
                    );
                    code.push_str("\t\t\treturn err\n");
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = item\n", field_name));
                    code.push_str("\t\t// Calculate consumed felts to update offset\n");
                    code.push_str("\t\tif itemData, err := item.MarshalCairo(); err != nil {\n");
                    code.push_str("\t\t\treturn err\n");
                    code.push_str("\t\t} else {\n");
                    code.push_str("\t\t\toffset += len(itemData)\n");
                    code.push_str("\t\t}\n");
                }
            }
            _ => {
                code.push_str("\t\t// TODO: Handle unknown token type in array unmarshal\n");
            }
        }

        code.push_str("\t}\n\n");
        code
    }

    /// Generates marshal code for tuple fields  
    fn generate_tuple_marshal_code(
        &self,
        field_name: &str,
        tuple: &cainome_parser::tokens::Tuple,
    ) -> String {
        let mut code = format!("\t// Tuple field {}: marshal each sub-field\n", field_name);

        for (index, inner_token) in tuple.inners.iter().enumerate() {
            let field_access = format!("s.{}.Field{}", field_name, index);
            match inner_token {
                Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                    "felt" | "core::felt252" => {
                        code.push_str(&format!("\tresult = append(result, {})\n", field_access));
                    }
                    "core::bool" => {
                        code.push_str(&format!(
                            "\tresult = append(result, cainome.FeltFromBool({}))\n",
                            field_access
                        ));
                    }
                    "core::integer::u8"
                    | "core::integer::u16"
                    | "core::integer::u32"
                    | "core::integer::u64"
                    | "core::integer::usize" => {
                        code.push_str(&format!(
                            "\tresult = append(result, cainome.FeltFromUint(uint64({})))\n",
                            field_access
                        ));
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        code.push_str(&format!(
                            "\tresult = append(result, cainome.FeltFromBigInt({}))\n",
                            field_access
                        ));
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                    | "core::integer::i64" => {
                        code.push_str(&format!(
                            "\tresult = append(result, cainome.FeltFromUint(uint64({})))\n",
                            field_access
                        ));
                    }
                    "core::starknet::contract_address::ContractAddress"
                    | "core::starknet::class_hash::ClassHash" => {
                        code.push_str(&format!("\tresult = append(result, {})\n", field_access));
                    }
                    _ => {
                        code.push_str(&format!(
                            "\t// TODO: Handle unknown basic type in tuple field {}\n",
                            index
                        ));
                    }
                },
                Token::Composite(composite) => {
                    if composite.is_builtin() {
                        match composite.type_path_no_generic().as_str() {
                            "core::integer::u256" => {
                                code.push_str(&format!(
                                    "\tresult = append(result, cainome.FeltFromBigInt({}))\n",
                                    field_access
                                ));
                            }
                            "core::starknet::eth_address::EthAddress" => {
                                code.push_str(&format!(
                                    "\tresult = append(result, cainome.FeltFromBytes({}[:]))\n",
                                    field_access
                                ));
                            }
                            "core::byte_array::ByteArray" => {
                                code.push_str(&format!(
                                    "\tif field{}_data, err := cainome.NewCairoByteArray({}).MarshalCairo(); err != nil {{\n\t\treturn nil, fmt.Errorf(\"failed to marshal ByteArray tuple field {}: %w\", err)\n\t}} else {{\n\t\tresult = append(result, field{}_data...)\n\t}}\n",
                                    index, field_access, index, index
                                ));
                            }
                            _ => {
                                code.push_str(&format!("\t// TODO: Handle unknown builtin composite type in tuple field {}\n", index));
                            }
                        }
                    } else {
                        // Custom struct/enum - use CairoMarshaler
                        code.push_str(&format!(
                            "\tif fieldData, err := {}.MarshalCairo(); err != nil {{\n",
                            field_access
                        ));
                        code.push_str("\t\treturn nil, err\n");
                        code.push_str("\t} else {\n");
                        code.push_str("\t\tresult = append(result, fieldData...)\n");
                        code.push_str("\t}\n");
                    }
                }
                Token::Array(_) => {
                    code.push_str(&format!(
                        "\t// TODO: Handle array type in tuple field {}\n",
                        index
                    ));
                }
                _ => {
                    code.push_str(&format!(
                        "\t// TODO: Handle unknown token type in tuple field {}\n",
                        index
                    ));
                }
            }
        }

        code
    }

    /// Generates unmarshal code for tuple fields
    fn generate_tuple_unmarshal_code(
        &self,
        field_name: &str,
        tuple: &cainome_parser::tokens::Tuple,
    ) -> String {
        let mut code = format!(
            "\t// Tuple field {}: unmarshal each sub-field\n",
            field_name
        );

        for (index, inner_token) in tuple.inners.iter().enumerate() {
            let field_access = format!("s.{}.Field{}", field_name, index);
            match inner_token {
                Token::CoreBasic(core_basic) => {
                    match core_basic.type_path.as_str() {
                        "felt" | "core::felt252" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!("\t{} = data[offset]\n", field_access));
                            code.push_str("\toffset++\n");
                        }
                        "core::bool" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = cainome.BoolFromFelt(data[offset])\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::u8" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = uint8(cainome.UintFromFelt(data[offset]))\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::u16" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = uint16(cainome.UintFromFelt(data[offset]))\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::u32" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = uint32(cainome.UintFromFelt(data[offset]))\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::u64" | "core::integer::usize" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = cainome.UintFromFelt(data[offset])\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::u128" | "core::integer::i128" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = cainome.BigIntFromFelt(data[offset])\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::i8" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = int8(cainome.UintFromFelt(data[offset]))\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::i16" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = int16(cainome.UintFromFelt(data[offset]))\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::i32" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = int32(cainome.UintFromFelt(data[offset]))\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::integer::i64" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!(
                                "\t{} = int64(cainome.UintFromFelt(data[offset]))\n",
                                field_access
                            ));
                            code.push_str("\toffset++\n");
                        }
                        "core::starknet::contract_address::ContractAddress"
                        | "core::starknet::class_hash::ClassHash" => {
                            code.push_str("\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                            code.push_str("\t}\n");
                            code.push_str(&format!("\t{} = data[offset]\n", field_access));
                            code.push_str("\toffset++\n");
                        }
                        _ => {
                            code.push_str(&format!("\t// TODO: Handle unknown basic type in tuple field {} element {}\n", field_name, index));
                        }
                    }
                }
                Token::Composite(composite) => {
                    if composite.is_builtin() {
                        match composite.type_path_no_generic().as_str() {
                            "core::integer::u256" => {
                                code.push_str("\tif offset >= len(data) {\n");
                                code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                                code.push_str("\t}\n");
                                code.push_str(&format!(
                                    "\t{} = cainome.BigIntFromFelt(data[offset])\n",
                                    field_access
                                ));
                                code.push_str("\toffset++\n");
                            }
                            "core::starknet::eth_address::EthAddress" => {
                                code.push_str("\tif offset >= len(data) {\n");
                                code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                                code.push_str("\t}\n");
                                code.push_str(&format!(
                                     "\tethBytes{} := data[offset].Bytes()\n\tcopy({}[:], ethBytes{}[:])\n",
                                     index, field_access, index
                                 ));
                                code.push_str("\toffset++\n");
                            }
                            "core::byte_array::ByteArray" => {
                                code.push_str(&format!(
                                    "\t// ByteArray unmarshaling for tuple field {} element {}\n\tif byteArrayLength{} := len(data) - offset; byteArrayLength{} > 0 {{\n\t\tbyteArray{} := &cainome.CairoByteArray{{}}\n\t\tif err := byteArray{}.UnmarshalCairo(data[offset:]); err != nil {{\n\t\t\treturn fmt.Errorf(\"failed to unmarshal ByteArray tuple field {} element {}: %w\", err)\n\t\t}}\n\t\t{} = byteArray{}.ToBytes()\n\t\t// TODO: Update offset based on consumed data for ByteArray tuple field {} element {}\n\t}}\n",
                                    field_name, index, index, index, index, index, field_name, index, field_access, index, field_name, index
                                ));
                            }
                            _ => {
                                code.push_str(&format!("\t// TODO: Handle unknown builtin composite type in tuple field {} element {}\n", field_name, index));
                            }
                        }
                    } else {
                        // Custom struct/enum - use CairoMarshaler
                        code.push_str(&format!(
                            "\tif err := {}.UnmarshalCairo(data[offset:]); err != nil {{\n",
                            field_access
                        ));
                        code.push_str("\t\treturn err\n");
                        code.push_str("\t}\n");
                        code.push_str("\t// Calculate consumed felts to update offset\n");
                        code.push_str(&format!(
                            "\tif itemData, err := {}.MarshalCairo(); err != nil {{\n",
                            field_access
                        ));
                        code.push_str("\t\treturn err\n");
                        code.push_str("\t} else {\n");
                        code.push_str("\t\toffset += len(itemData)\n");
                        code.push_str("\t}\n");
                    }
                }
                Token::Array(_) => {
                    code.push_str(&format!(
                        "\t// TODO: Handle array type in tuple field {} element {}\n",
                        field_name, index
                    ));
                }
                _ => {
                    code.push_str(&format!(
                        "\t// TODO: Handle unknown token type in tuple field {} element {}\n",
                        field_name, index
                    ));
                }
            }
        }

        code.push('\n');
        code
    }

    /// Generates response deserialization code for tuple types
    fn generate_tuple_response_deserialization(
        &self,
        tuple: &cainome_parser::tokens::Tuple,
        go_type: &str,
    ) -> String {
        if tuple.inners.is_empty() {
            return "\treturn struct{}{}, nil\n".to_string();
        }

        let mut code = format!("\tvar result {}\n", go_type);
        code.push_str("\toffset := 0\n\n");

        for (index, inner_token) in tuple.inners.iter().enumerate() {
            let field_access = format!("result.Field{}", index);

            code.push_str("\tif offset >= len(response) {\n");
            code.push_str(&format!(
                "\t\treturn {}{{}}, fmt.Errorf(\"insufficient data for tuple field {}\")\n",
                go_type, index
            ));
            code.push_str("\t}\n");

            match inner_token {
                Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                    "felt" | "core::felt252" => {
                        code.push_str(&format!("\t{} = response[offset]\n", field_access));
                    }
                    "core::bool" => {
                        code.push_str(&format!(
                            "\t{} = cainome.BoolFromFelt(response[offset])\n",
                            field_access
                        ));
                    }
                    "core::integer::u8" => {
                        code.push_str(&format!(
                            "\t{} = uint8(cainome.UintFromFelt(response[offset]))\n",
                            field_access
                        ));
                    }
                    "core::integer::u16" => {
                        code.push_str(&format!(
                            "\t{} = uint16(cainome.UintFromFelt(response[offset]))\n",
                            field_access
                        ));
                    }
                    "core::integer::u32" => {
                        code.push_str(&format!(
                            "\t{} = uint32(cainome.UintFromFelt(response[offset]))\n",
                            field_access
                        ));
                    }
                    "core::integer::u64" | "core::integer::usize" => {
                        code.push_str(&format!(
                            "\t{} = cainome.UintFromFelt(response[offset])\n",
                            field_access
                        ));
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        code.push_str(&format!(
                            "\t{} = cainome.BigIntFromFelt(response[offset])\n",
                            field_access
                        ));
                    }
                    "core::integer::i8" => {
                        code.push_str(&format!(
                            "\t{} = int8(cainome.IntFromFelt(response[offset]))\n",
                            field_access
                        ));
                    }
                    "core::integer::i16" => {
                        code.push_str(&format!(
                            "\t{} = int16(cainome.IntFromFelt(response[offset]))\n",
                            field_access
                        ));
                    }
                    "core::integer::i32" => {
                        code.push_str(&format!(
                            "\t{} = int32(cainome.IntFromFelt(response[offset]))\n",
                            field_access
                        ));
                    }
                    "core::integer::i64" => {
                        code.push_str(&format!(
                            "\t{} = cainome.IntFromFelt(response[offset])\n",
                            field_access
                        ));
                    }
                    "core::integer::u256" => {
                        code.push_str(&format!(
                            "\t{} = cainome.BigIntFromFelt(response[offset])\n",
                            field_access
                        ));
                    }
                    _ => {
                        code.push_str(&format!(
                            "\t// TODO: Handle core basic type {} for tuple field {}\n",
                            core_basic.type_path, index
                        ));
                    }
                },
                Token::NonZero(non_zero) => {
                    // NonZero types are just the inner type
                    let inner_type = &non_zero.inner;
                    if let Token::CoreBasic(core_basic) = inner_type.as_ref() {
                        match core_basic.type_path.as_str() {
                            "felt" | "core::felt252" => {
                                code.push_str(&format!("\t{} = response[offset]\n", field_access));
                            }
                            _ => {
                                code.push_str(&format!(
                                    "\t// TODO: Handle NonZero<{}> for tuple field {}\n",
                                    core_basic.type_path, index
                                ));
                            }
                        }
                    }
                }
                Token::Composite(composite) => {
                    if composite.type_path == "core::integer::u256" {
                        code.push_str(&format!(
                            "\t{} = cainome.BigIntFromFelt(response[offset])\n",
                            field_access
                        ));
                    } else {
                        code.push_str(&format!(
                            "\t// TODO: Handle composite {} for tuple field {}\n",
                            composite.type_path, index
                        ));
                    }
                }
                _ => {
                    code.push_str(&format!(
                        "\t// TODO: Handle token type {:?} for tuple field {}\n",
                        inner_token, index
                    ));
                }
            }
            code.push_str("\toffset++\n\n");
        }

        code.push_str("\treturn result, nil\n");
        code
    }

    /// Generates marshal code for Option fields
    fn generate_option_marshal_code(&self, field_name: &str, inner_token: &Token) -> String {
        let mut code = format!(
            "\t// Option field {}: check for nil and marshal accordingly\n",
            field_name
        );
        code.push_str(&format!("\tif s.{} != nil {{\n", field_name));
        code.push_str("\t\t// Some variant: discriminant 0 + value\n");
        code.push_str("\t\tresult = append(result, cainome.FeltFromUint(0))\n");

        // Marshal the inner value based on its type
        match inner_token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    code.push_str(&format!("\t\tresult = append(result, *s.{})\n", field_name));
                }
                "core::bool" => {
                    code.push_str(&format!(
                        "\t\tresult = append(result, cainome.FeltFromBool(*s.{}))\n",
                        field_name
                    ));
                }
                "core::integer::u8"
                | "core::integer::u16"
                | "core::integer::u32"
                | "core::integer::u64"
                | "core::integer::usize" => {
                    code.push_str(&format!(
                        "\t\tresult = append(result, cainome.FeltFromUint(uint64(*s.{})))\n",
                        field_name
                    ));
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str(&format!(
                        "\t\tresult = append(result, cainome.FeltFromBigInt(*s.{}))\n",
                        field_name
                    ));
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                | "core::integer::i64" => {
                    code.push_str(&format!(
                        "\t\tresult = append(result, cainome.FeltFromUint(uint64(*s.{})))\n",
                        field_name
                    ));
                }
                "core::starknet::contract_address::ContractAddress"
                | "core::starknet::class_hash::ClassHash" => {
                    code.push_str(&format!("\t\tresult = append(result, *s.{})\n", field_name));
                }
                _ => {
                    code.push_str(&format!(
                        "\t\t// TODO: Handle unknown basic type in Option field {}\n",
                        field_name
                    ));
                }
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            code.push_str(&format!(
                                "\t\tresult = append(result, cainome.FeltFromBigInt(*s.{}))\n",
                                field_name
                            ));
                        }
                        _ => {
                            code.push_str(&format!("\t\t// TODO: Handle unknown builtin composite type in Option field {}\n", field_name));
                        }
                    }
                } else {
                    // Custom struct/enum - use CairoMarshaler
                    code.push_str(&format!(
                        "\t\tif fieldData, err := s.{}.MarshalCairo(); err != nil {{\n",
                        field_name
                    ));
                    code.push_str("\t\t\treturn nil, err\n");
                    code.push_str("\t\t} else {\n");
                    code.push_str("\t\t\tresult = append(result, fieldData...)\n");
                    code.push_str("\t\t}\n");
                }
            }
            _ => {
                code.push_str(&format!(
                    "\t\t// TODO: Handle unknown token type in Option field {}\n",
                    field_name
                ));
            }
        }

        code.push_str("\t} else {\n");
        code.push_str("\t\t// None variant: discriminant 1 (no additional data)\n");
        code.push_str("\t\tresult = append(result, cainome.FeltFromUint(1))\n");
        code.push_str("\t}\n");
        code
    }

    /// Generates unmarshal code for Option fields
    fn generate_option_unmarshal_code(&self, field_name: &str, inner_token: &Token) -> String {
        let mut code = format!(
            "\t// Option field {}: read discriminant then value if Some\n",
            field_name
        );
        code.push_str("\tif offset >= len(data) {\n");
        code.push_str(&format!(
            "\t\treturn fmt.Errorf(\"insufficient data for Option field {} discriminant\")\n",
            field_name
        ));
        code.push_str("\t}\n");
        code.push_str("\tdiscriminant := cainome.UintFromFelt(data[offset])\n");
        code.push_str("\toffset++\n");
        code.push_str("\tif discriminant == 0 {\n");
        code.push_str("\t\t// Some variant: read the value\n");

        // Get the Go type for the inner value
        let inner_type = self.token_to_go_type(inner_token);
        code.push_str(&format!("\t\tvar value {}\n", inner_type));

        // Unmarshal the inner value based on its type
        match inner_token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = data[offset]\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::bool" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = cainome.BoolFromFelt(data[offset])\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = uint8(cainome.UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = uint16(cainome.UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = uint32(cainome.UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u64" | "core::integer::usize" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = cainome.UintFromFelt(data[offset])\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = cainome.BigIntFromFelt(data[offset])\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int8(cainome.UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int16(cainome.UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int32(cainome.UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i64" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int64(cainome.UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::starknet::contract_address::ContractAddress"
                | "core::starknet::class_hash::ClassHash" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = data[offset]\n");
                    code.push_str("\t\toffset++\n");
                }
                _ => {
                    code.push_str(&format!(
                        "\t\t// TODO: Handle unknown basic type in Option field {} unmarshal\n",
                        field_name
                    ));
                }
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            code.push_str("\t\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                            code.push_str("\t\t}\n");
                            code.push_str("\t\tvalue = cainome.BigIntFromFelt(data[offset])\n");
                            code.push_str("\t\toffset++\n");
                        }
                        _ => {
                            code.push_str(&format!("\t\t// TODO: Handle unknown builtin composite type in Option field {} unmarshal\n", field_name));
                        }
                    }
                } else {
                    // Custom struct/enum - use CairoMarshaler
                    code.push_str(
                        "\t\tif err := value.UnmarshalCairo(data[offset:]); err != nil {\n",
                    );
                    code.push_str("\t\t\treturn err\n");
                    code.push_str("\t\t}\n");
                    code.push_str("\t\t// Calculate consumed felts to update offset\n");
                    code.push_str("\t\tif itemData, err := value.MarshalCairo(); err != nil {\n");
                    code.push_str("\t\t\treturn err\n");
                    code.push_str("\t\t} else {\n");
                    code.push_str("\t\t\toffset += len(itemData)\n");
                    code.push_str("\t\t}\n");
                }
            }
            _ => {
                code.push_str(&format!(
                    "\t\t// TODO: Handle unknown token type in Option field {} unmarshal\n",
                    field_name
                ));
            }
        }

        code.push_str(&format!("\t\ts.{} = &value\n", field_name));
        code.push_str("\t} else {\n");
        code.push_str("\t\t// None variant\n");
        code.push_str(&format!("\t\ts.{} = nil\n", field_name));
        code.push_str("\t}\n\n");
        code
    }

    /// Generates marshal code for Result fields
    fn generate_result_marshal_code(
        &self,
        field_name: &str,
        _ok_token: &Token,
        _err_token: &Token,
    ) -> String {
        format!("\t// Result field {}: marshal using Cairo Result pattern\n\tif fieldData, err := s.{}.MarshalCairo(); err != nil {{\n\t\treturn nil, err\n\t}} else {{\n\t\tresult = append(result, fieldData...)\n\t}}\n", field_name, field_name)
    }

    /// Generates unmarshal code for Result fields
    fn generate_result_unmarshal_code(
        &self,
        field_name: &str,
        _ok_token: &Token,
        _err_token: &Token,
    ) -> String {
        format!("\t// Result field {}: unmarshal using Cairo Result pattern\n\tif err := s.{}.UnmarshalCairo(data[offset:]); err != nil {{\n\t\treturn err\n\t}}\n\t// TODO: Update offset based on consumed data\n\n", field_name, field_name)
    }

    /// Generates marshal code for a single field
    fn generate_field_marshal_code(&self, field_name: &str, token: &Token) -> String {
        match token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    format!("\tresult = append(result, s.{})\n", field_name)
                }
                "core::bool" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromBool(s.{}))\n",
                        field_name
                    )
                }
                "core::integer::u8"
                | "core::integer::u16"
                | "core::integer::u32"
                | "core::integer::u64"
                | "core::integer::usize" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromUint(uint64(s.{})))\n",
                        field_name
                    )
                }
                "core::integer::u128" | "core::integer::i128" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromBigInt(s.{}))\n",
                        field_name
                    )
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                | "core::integer::i64" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromUint(uint64(s.{})))\n",
                        field_name
                    )
                }
                "core::starknet::contract_address::ContractAddress"
                | "core::starknet::class_hash::ClassHash" => {
                    format!("\tresult = append(result, s.{})\n", field_name)
                }
                "core::bytes_31::bytes31" => {
                    format!(
                        "\tresult = append(result, cainome.FeltFromBytes({}.Bytes()))\n",
                        field_name
                    )
                }
                _ => format!(
                    "\t// TODO: Handle unknown core basic type for field {}\n",
                    field_name
                ),
            },
            Token::Array(array) => self.generate_array_marshal_code(field_name, &array.inner),
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::byte_array::ByteArray" => {
                            format!(
                                "\tif {}_data, err := cainome.NewCairoByteArray(s.{}).MarshalCairo(); err != nil {{\n\t\treturn nil, fmt.Errorf(\"failed to marshal ByteArray field {}: %w\", err)\n\t}} else {{\n\t\tresult = append(result, {}_data...)\n\t}}\n",
                                field_name, field_name, field_name, field_name
                            )
                        }
                        "core::integer::u256" => {
                            format!(
                                "\tresult = append(result, cainome.FeltFromBigInt(s.{}))\n",
                                field_name
                            )
                        }
                        "core::starknet::eth_address::EthAddress" => {
                            format!(
                                "\tresult = append(result, cainome.FeltFromBytes(s.{}[:]))\n",
                                field_name
                            )
                        }
                        _ => format!(
                            "\t// TODO: Handle builtin composite {} for field {}\n",
                            composite.type_path_no_generic(),
                            field_name
                        ),
                    }
                } else {
                    format!("\t// Struct field {}: marshal using CairoMarshaler\n\tif fieldData, err := s.{}.MarshalCairo(); err != nil {{\n\t\treturn nil, err\n\t}} else {{\n\t\tresult = append(result, fieldData...)\n\t}}\n", field_name, field_name)
                }
            }
            Token::Option(option) => self.generate_option_marshal_code(field_name, &option.inner),
            Token::Result(result) => {
                self.generate_result_marshal_code(field_name, &result.inner, &result.error)
            }
            Token::NonZero(non_zero) => {
                // NonZero types are just wrappers, marshal the inner type directly
                self.generate_field_marshal_code(field_name, &non_zero.inner)
            }
            Token::Tuple(tuple) => self.generate_tuple_marshal_code(field_name, tuple),
            _ => format!(
                "\t// TODO: Handle unknown token type for field {}\n",
                field_name
            ),
        }
    }

    /// Generates unmarshal code for a single field
    fn generate_field_unmarshal_code(&self, field_name: &str, token: &Token) -> String {
        match token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = data[offset]\n\toffset++\n\n", field_name, field_name)
                }
                "core::bool" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = cainome.BoolFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u8" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = uint8(cainome.UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u16" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = uint16(cainome.UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u32" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = uint32(cainome.UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u64" | "core::integer::usize" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = cainome.UintFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u128" | "core::integer::i128" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = cainome.BigIntFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i8" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int8(cainome.UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i16" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int16(cainome.UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i32" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int32(cainome.UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i64" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int64(cainome.UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = data[offset]\n\toffset++\n\n", field_name, field_name)
                }
                "core::bytes_31::bytes31" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\tbytes31Data := data[offset].Bytes()\n\tcopy(s.{}[:], bytes31Data[:])\n\toffset++\n\n", field_name, field_name)
                }
                _ => format!("\t// TODO: Handle unknown core basic type for field {} unmarshal\n\t_ = offset // Suppress unused variable warning\n", field_name),
            },
            Token::Array(array) => {
                self.generate_array_unmarshal_code(field_name, &array.inner)
            }
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = cainome.BigIntFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
                        }
                        "core::starknet::eth_address::EthAddress" => {
                            format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\tethBytes := data[offset].Bytes()\n\tcopy(s.{}[:], ethBytes[:])\n\toffset++\n\n", field_name, field_name)
                        }
                        "core::byte_array::ByteArray" => {
                            format!("\t// ByteArray unmarshaling for field {}\n\tif byteArrayLength := len(data) - offset; byteArrayLength > 0 {{\n\t\tbyteArray := &cainome.CairoByteArray{{}}\n\t\tif consumed, err := byteArray.UnmarshalCairoWithLength(data[offset:]); err != nil {{\n\t\t\treturn fmt.Errorf(\"failed to unmarshal ByteArray field {}: %w\", err)\n\t\t}} else {{\n\t\t\ts.{} = byteArray.ToBytes()\n\t\t\toffset += consumed\n\t\t}}\n\t}}\n\n", field_name, field_name, field_name)
                        }
                        _ => format!("\t// TODO: Handle builtin composite {} for field {} unmarshal\n\t_ = offset // Suppress unused variable warning\n", composite.type_path_no_generic(), field_name),
                    }
                } else {
                    format!("\t// Struct field {}: unmarshal using CairoMarshaler\n\tif err := s.{}.UnmarshalCairo(data[offset:]); err != nil {{\n\t\treturn err\n\t}}\n\t// TODO: Update offset based on consumed data\n\n", field_name, field_name)
                }
            }
            Token::Option(option) => {
                self.generate_option_unmarshal_code(field_name, &option.inner)
            }
            Token::Result(result) => {
                self.generate_result_unmarshal_code(field_name, &result.inner, &result.error)
            }
            Token::NonZero(non_zero) => {
                // NonZero types are just wrappers, unmarshal the inner type directly
                self.generate_field_unmarshal_code(field_name, &non_zero.inner)
            }
            Token::Tuple(tuple) => {
                self.generate_tuple_unmarshal_code(field_name, tuple)
            }
            _ => format!("\t// TODO: Handle unknown token type for field {} unmarshal\n\t_ = offset // Suppress unused variable warning\n", field_name),
        }
    }

    /// Generates Go function definition for a Cairo contract function
    fn generate_function(&self, function: &Function, contract_name: &str) -> String {
        let is_view = function.state_mutability == StateMutability::View;

        if is_view {
            // Generate view function for Reader struct
            self.generate_reader_function(function, contract_name)
        } else {
            // Generate invoke function for Writer struct
            self.generate_writer_function(function, contract_name)
        }
    }

    /// Generates a view function for the Reader struct
    fn generate_reader_function(&self, function: &Function, contract_name: &str) -> String {
        let func_name = function.name.to_case(Case::Pascal);
        let sanitized_contract_name = self.sanitize_go_identifier(contract_name);
        let receiver_name = format!("{}Reader", sanitized_contract_name.to_case(Case::Snake));
        let struct_name = format!("{}Reader", sanitized_contract_name.to_case(Case::Pascal));

        // Generate parameters
        let mut params = Vec::new();
        params.push("ctx context.Context".to_string());

        // Add contract function parameters
        for (param_name, param_token) in &function.inputs {
            let go_type = self.token_to_go_param_type(param_token);
            let param_snake = param_name.to_case(Case::Snake);
            let safe_param_name = self.generate_safe_param_name(&param_snake);

            params.push(format!("{} {}", safe_param_name, go_type));
        }

        // Add opts parameter for view functions
        params.push("opts *cainome.CallOpts".to_string());

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

        let mut func_def = format!(
            "func ({} *{}) {}({}) {} {{\n",
            receiver_name.to_case(Case::Snake),
            struct_name,
            func_name,
            params.join(", "),
            return_str
        );

        // Generate call method for view functions
        func_def
            .push_str(&self.generate_call_method(function, &receiver_name.to_case(Case::Snake)));
        func_def.push_str("}\n\n");
        func_def
    }

    /// Generates an invoke function for the Writer struct
    fn generate_writer_function(&self, function: &Function, contract_name: &str) -> String {
        let func_name = function.name.to_case(Case::Pascal);
        let sanitized_contract_name = self.sanitize_go_identifier(contract_name);
        let receiver_name = format!("{}Writer", sanitized_contract_name.to_case(Case::Snake));
        let struct_name = format!("{}Writer", sanitized_contract_name.to_case(Case::Pascal));

        // Generate parameters
        let mut params = Vec::new();
        params.push("ctx context.Context".to_string());

        // Add contract function parameters
        for (param_name, param_token) in &function.inputs {
            let go_type = self.token_to_go_param_type(param_token);
            let param_snake = param_name.to_case(Case::Snake);
            let safe_param_name = self.generate_safe_param_name(&param_snake);
            params.push(format!("{} {}", safe_param_name, go_type));
        }

        // Add opts parameter for invoke functions
        params.push("opts *cainome.InvokeOpts".to_string());

        // Generate return types - invoke functions can return values plus transaction hash
        let mut returns = Vec::new();
        for (i, output_token) in function.outputs.iter().enumerate() {
            let go_type = self.token_to_go_type(output_token);
            if function.outputs.len() == 1 {
                returns.push(go_type);
            } else {
                returns.push(format!("ret{} {}", i, go_type));
            }
        }

        // Add transaction hash and error return
        returns.push("*felt.Felt".to_string()); // Transaction hash
        returns.push("error".to_string());

        let return_str = if returns.len() == 2 && function.outputs.is_empty() {
            "(*felt.Felt, error)".to_string() // Just tx hash and error for functions with no outputs
        } else {
            format!("({})", returns.join(", "))
        };

        let mut func_def = format!(
            "func ({} *{}) {}({}) {} {{\n",
            receiver_name.to_case(Case::Snake),
            struct_name,
            func_name,
            params.join(", "),
            return_str
        );

        // Generate invoke method for state-changing functions
        func_def
            .push_str(&self.generate_invoke_method(function, &receiver_name.to_case(Case::Snake)));
        func_def.push_str("}\n\n");
        func_def
    }

    /// Generate the body for a view function call
    fn generate_call_method(&self, function: &Function, receiver_name: &str) -> String {
        let mut method_body = String::new();

        // Special case: if function has no outputs, just return nil immediately
        if function.outputs.is_empty() {
            method_body.push_str("\treturn nil\n");
            return method_body;
        }

        // Handle call options
        method_body.push_str("\t// Setup call options\n");
        method_body.push_str("\tif opts == nil {\n");
        method_body.push_str("\t\topts = &cainome.CallOpts{}\n");
        method_body.push_str("\t}\n");
        method_body.push_str("\tvar blockID rpc.BlockID\n");
        method_body.push_str("\tif opts.BlockID != nil {\n");
        method_body.push_str("\t\tblockID = *opts.BlockID\n");
        method_body.push_str("\t} else {\n");
        method_body.push_str("\t\tblockID = rpc.BlockID{Tag: \"latest\"}\n");
        method_body.push_str("\t}\n\n");

        // Build calldata array
        if function.inputs.is_empty() {
            method_body.push_str("\t// No parameters required\n");
            method_body.push_str("\tcalldata := []*felt.Felt{}\n\n");
        } else {
            method_body.push_str("\t// Serialize parameters to calldata\n");
            method_body.push_str("\tcalldata := []*felt.Felt{}\n");
            for (param_name, param_token) in &function.inputs {
                let param_snake = param_name.to_case(Case::Snake);
                let safe_param_name = self.generate_safe_param_name(&param_snake);

                // Generate serialization code based on parameter type
                match param_token {
                    Token::Option(option) => {
                        // Handle Option types specially
                        match option.inner.as_ref() {
                            Token::Composite(composite)
                                if composite.r#type == CompositeType::Enum =>
                            {
                                // Option of enum interface needs special handling
                                method_body
                                    .push_str(&format!("\tif {} != nil {{\n", safe_param_name));
                                method_body.push_str("\t\t// Some variant\n");
                                method_body.push_str(
                                    "\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n",
                                );
                                method_body.push_str(&format!(
                                    "\t\tif {}_data, err := (*{}).MarshalCairo(); err != nil {{\n",
                                    safe_param_name, safe_param_name
                                ));
                                let zero_returns = self.generate_zero_returns(function);
                                let error_return = if zero_returns.is_empty() {
                                    "err".to_string()
                                } else {
                                    format!("{}, err", zero_returns)
                                };
                                method_body.push_str(&format!("\t\t\treturn {}\n", error_return));
                                method_body.push_str("\t\t} else {\n");
                                method_body.push_str(&format!(
                                    "\t\t\tcalldata = append(calldata, {}_data...)\n",
                                    safe_param_name
                                ));
                                method_body.push_str("\t\t}\n");
                                method_body.push_str("\t} else {\n");
                                method_body.push_str("\t\t// None variant\n");
                                method_body.push_str(
                                    "\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n",
                                );
                                method_body.push_str("\t}\n");
                            }
                            _ => {
                                // For other Option types, use basic serialization
                                let serialization_code = self
                                    .generate_basic_type_serialization_with_context(
                                        param_token,
                                        &safe_param_name,
                                        function,
                                    );
                                method_body.push_str(&serialization_code);
                            }
                        }
                    }
                    _ if self.is_complex_type(param_token) => {
                        method_body.push_str(&format!(
                            "\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n",
                            safe_param_name, safe_param_name
                        ));
                        let zero_returns = self.generate_zero_returns(function);
                        let error_return = if zero_returns.is_empty() {
                            "err".to_string()
                        } else {
                            format!("{}, err", zero_returns)
                        };
                        method_body.push_str(&format!("\t\treturn {}\n", error_return));
                        method_body.push_str("\t} else {\n");
                        method_body.push_str(&format!(
                            "\t\tcalldata = append(calldata, {}_data...)\n",
                            safe_param_name
                        ));
                        method_body.push_str("\t}\n");
                    }
                    _ => {
                        // For basic types, use direct serialization
                        let serialization_code = self
                            .generate_basic_type_serialization_with_context(
                                param_token,
                                &safe_param_name,
                                function,
                            );
                        method_body.push_str(&serialization_code);
                    }
                }
            }
            method_body.push('\n');
        }

        // Generate RPC call
        method_body.push_str("\t// Make the contract call\n");
        method_body.push_str("\tfunctionCall := rpc.FunctionCall{\n");
        method_body.push_str(&format!(
            "\t\tContractAddress:    {}.contractAddress,\n",
            receiver_name
        ));
        method_body.push_str(&format!(
            "\t\tEntryPointSelector: utils.GetSelectorFromNameFelt(\"{}\"),\n",
            function.name
        ));
        method_body.push_str("\t\tCalldata:           calldata,\n");
        method_body.push_str("\t}\n\n");

        method_body.push_str(&format!(
            "\tresponse, err := {}.provider.Call(ctx, functionCall, blockID)\n",
            receiver_name
        ));
        method_body.push_str("\tif err != nil {\n");
        if function.outputs.is_empty() {
            method_body.push_str("\t\treturn err\n");
        } else if function.outputs.len() == 1 {
            let return_type = self.token_to_go_type(&function.outputs[0]);
            let zero_value = self.generate_zero_value(&return_type);
            method_body.push_str(&format!("\t\treturn {}, err\n", zero_value));
        } else {
            // Multiple return values - return zero values for each plus error
            let zero_returns: Vec<String> = function
                .outputs
                .iter()
                .map(|output| {
                    let return_type = self.token_to_go_type(output);
                    self.generate_zero_value(&return_type)
                })
                .collect();
            method_body.push_str(&format!("\t\treturn {}, err\n", zero_returns.join(", ")));
        }
        method_body.push_str("\t}\n\n");

        // Handle response deserialization
        if function.outputs.is_empty() {
            method_body.push_str("\treturn nil\n");
        } else if function.outputs.len() == 1 {
            method_body.push_str("\t// Deserialize response to proper type\n");
            method_body.push_str("\tif len(response) == 0 {\n");
            let return_type = self.token_to_go_type(&function.outputs[0]);
            let zero_value = self.generate_zero_value(&return_type);
            method_body.push_str(&format!(
                "\t\treturn {}, fmt.Errorf(\"empty response\")\n",
                zero_value
            ));
            method_body.push_str("\t}\n");

            // Handle deserialization based on output type
            if return_type == "*felt.Felt" {
                method_body.push_str("\treturn response[0], nil\n");
            } else if matches!(&function.outputs[0], Token::Option(_)) {
                // Option types always use their specific deserialization logic
                let deserialization_code =
                    self.generate_basic_type_deserialization(&function.outputs[0], &return_type);
                method_body.push_str(&deserialization_code);
            } else if matches!(&function.outputs[0], Token::Composite(composite) if composite.r#type == CompositeType::Enum)
            {
                // Enum interfaces need special deserialization logic
                let enum_name = function.outputs[0].type_name();
                let deserialization_code = self.generate_enum_interface_deserialization(
                    &return_type,
                    &enum_name,
                    &function.outputs[0],
                );
                method_body.push_str(&deserialization_code);
            } else if self.is_complex_type(&function.outputs[0]) {
                method_body.push_str(&format!("\tvar result {}\n", return_type));
                method_body.push_str("\tif err := result.UnmarshalCairo(response); err != nil {\n");
                method_body.push_str(&format!(
                    "\t\treturn {}, fmt.Errorf(\"failed to unmarshal response: %w\", err)\n",
                    zero_value
                ));
                method_body.push_str("\t}\n");
                method_body.push_str("\treturn result, nil\n");
            } else {
                // For basic types, direct conversion
                let deserialization_code =
                    self.generate_basic_type_deserialization(&function.outputs[0], &return_type);
                method_body.push_str(&deserialization_code);
            }
        } else {
            method_body.push_str("\t// Multiple return values - basic deserialization\n");
            method_body.push_str("\tif len(response) == 0 {\n");
            // Return proper zero values for multiple returns
            let zero_returns: Vec<String> = function
                .outputs
                .iter()
                .map(|output| {
                    let return_type = self.token_to_go_type(output);
                    self.generate_zero_value(&return_type)
                })
                .collect();
            method_body.push_str(&format!(
                "\t\treturn {}, fmt.Errorf(\"empty response\")\n",
                zero_returns.join(", ")
            ));
            method_body.push_str("\t}\n");
            method_body.push_str("\t// Basic deserialization for multiple return values\n");
            let return_values: Vec<String> = function
                .outputs
                .iter()
                .enumerate()
                .map(|(i, output)| {
                    let return_type = self.token_to_go_type(output);
                    if return_type == "*felt.Felt" && i < function.outputs.len() && i < 10 {
                        // For felt types, use response elements directly (up to 10 elements)
                        format!("response[{}]", i)
                    } else {
                        // For other types, return zero values for now
                        self.generate_zero_value(&return_type)
                    }
                })
                .collect();
            method_body.push_str(&format!("\treturn {}, nil\n", return_values.join(", ")));
        }

        method_body
    }

    /// Generate safe parameter name that doesn't conflict with package names
    fn generate_safe_param_name(&self, param_name: &str) -> String {
        match param_name {
            "felt" => "feltValue".to_string(),
            "rpc" => "rpcValue".to_string(),
            "big" => "bigValue".to_string(),
            "fmt" => "fmtValue".to_string(),
            "context" => "ctxValue".to_string(),
            _ => param_name.to_string(),
        }
    }

    /// Check if a token represents a complex type that needs CairoMarshaler
    fn is_complex_type(&self, token: &Token) -> bool {
        match token {
            Token::Composite(composite) => {
                // Built-in composite types like u256 should be treated as basic types
                if composite.is_builtin() {
                    false
                } else {
                    // Also check for specific Cairo built-in types that should be treated as basic
                    !matches!(
                        composite.type_path.as_str(),
                        "core::integer::u256"
                            | "core::byte_array::ByteArray"
                            | "core::starknet::eth_address::EthAddress"
                    )
                }
            }
            Token::NonZero(non_zero) => {
                // Check if NonZero wraps a basic type
                !matches!(non_zero.inner.as_ref(), Token::CoreBasic(core_basic) if core_basic.type_path == "felt" || core_basic.type_path == "core::felt252")
            }
            Token::Option(option) => {
                // Check if Option wraps a basic type
                match option.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        false
                    }
                    Token::CoreBasic(core_basic)
                        if matches!(
                            core_basic.type_path.as_str(),
                            "core::integer::u8"
                                | "core::integer::u16"
                                | "core::integer::u32"
                                | "core::integer::u64"
                        ) =>
                    {
                        false
                    }
                    _ => true,
                }
            }
            Token::Array(array) => {
                // Arrays are basic types when they contain basic elements
                match array.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        false
                    }
                    _ => false, // For now, treat most arrays as basic types
                }
            }
            Token::Tuple(_) => false, // Tuples are basic types - handle marshalling manually
            Token::Result(_) => true, // Results are complex types - use CairoMarshaler interface
            _ => false,
        }
    }

    /// Generate serialization code for basic types with invoke function context for proper error returns
    fn generate_basic_type_serialization_for_invoke(
        &self,
        token: &Token,
        param_name: &str,
        function: &Function,
    ) -> String {
        let zero_returns = self.generate_invoke_zero_returns(function);
        let error_return = format!(
            "{}, fmt.Errorf(\"failed to marshal {}: %w\", err)",
            zero_returns, param_name
        );

        match token {
            Token::CoreBasic(core_basic) => {
                match core_basic.type_path.as_str() {
                    "felt" | "core::felt252" => {
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    "core::bool" => {
                        format!("\tif {} {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}} else {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t}}\n", param_name)
                    }
                    "core::integer::u8"
                    | "core::integer::u16"
                    | "core::integer::u32"
                    | "core::integer::u64"
                    | "core::integer::usize" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromUint(uint64({})))\n",
                            param_name
                        )
                    }
                    "core::integer::u128" | "core::integer::i128" | "core::integer::u256" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n",
                            param_name
                        )
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                    | "core::integer::i64" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromInt(int64({})))\n",
                            param_name
                        )
                    }
                    "core::starknet::contract_address::ContractAddress"
                    | "core::starknet::class_hash::ClassHash" => {
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    "core::starknet::eth_address::EthAddress" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBytes({}[:]))\n",
                            param_name
                        )
                    }
                    "core::bytes_31::bytes31" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBytes({}.Bytes()))\n",
                            param_name
                        )
                    }
                    "core::byte_array::ByteArray" => {
                        format!("\tif {}_data, err := cainome.NewCairoByteArray({}).MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                    "[]byte" => {
                        // Handle Go []byte type - assuming it maps to ByteArray
                        format!("\tif {}_data, err := cainome.NewCairoByteArray({}).MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                    "()" => {
                        "".to_string() // Unit type - no serialization needed
                    }
                    _ => {
                        format!("\t// TODO: Add serialization for {}\n\tcalldata = append(calldata, {})\n", core_basic.type_path, param_name)
                    }
                }
            }
            Token::Composite(composite) => {
                // Handle built-in composite types like u256
                match composite.type_path.as_str() {
                    "core::integer::u256" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n",
                            param_name
                        )
                    }
                    "core::byte_array::ByteArray" => {
                        format!("\tif {}_data, err := cainome.NewCairoByteArray({}).MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                    "core::starknet::eth_address::EthAddress" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBytes({}[:]))\n",
                            param_name
                        )
                    }
                    _ => {
                        // For other composite types, try CairoMarshaler interface
                        format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                }
            }
            Token::Array(array) => {
                // Handle arrays based on their inner type
                match array.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // []*felt.Felt - serialize as length + elements
                        format!("\tcalldata = append(calldata, cainome.FeltFromUint(uint64(len({}))))\n\tcalldata = append(calldata, {}...)\n", param_name, param_name)
                    }
                    _ => {
                        // For arrays of complex types, iterate and marshal each element
                        format!("\t// Array of complex types: serialize length then elements\n\tcalldata = append(calldata, cainome.FeltFromUint(uint64(len({}))))\n\tfor _, item := range {} {{\n\t\tif item_data, err := item.MarshalCairo(); err != nil {{\n\t\t\treturn {}\n\t\t}} else {{\n\t\t\tcalldata = append(calldata, item_data...)\n\t\t}}\n\t}}\n", param_name, param_name, error_return)
                    }
                }
            }
            Token::NonZero(non_zero) => {
                // Handle NonZero types - check if the inner type is a basic type
                match non_zero.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // NonZero<felt> - can be directly appended as *felt.Felt
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    _ => {
                        // Other NonZero types - use CairoMarshaler interface
                        format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                }
            }
            Token::Option(option) => {
                // Handle Option types - check if the inner type is a basic type
                match option.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // Option<felt> - handle as **felt.Felt with nil check and direct append
                        format!("\tif {} != nil {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1)) // Some variant\n\t\tcalldata = append(calldata, *{})\n\t}} else {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0)) // None variant\n\t}}\n", param_name, param_name)
                    }
                    _ => {
                        // Other Option types - use CairoMarshaler interface
                        format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                }
            }
            Token::Result(_result) => {
                // For Result types, use MarshalCairo interface
                format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
            }
            Token::Tuple(tuple) => {
                // For Tuple types, serialize each field manually
                let mut code = String::new();
                for (i, inner_token) in tuple.inners.iter().enumerate() {
                    let field_access = format!("{}.Field{}", param_name, i);
                    match inner_token {
                        Token::CoreBasic(core_basic) => {
                            match core_basic.type_path.as_str() {
                                "felt" | "core::felt252" => {
                                    code.push_str(&format!(
                                        "\tcalldata = append(calldata, {})\n",
                                        field_access
                                    ));
                                }
                                "core::bool" => {
                                    code.push_str(&format!("\tif {} {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}} else {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t}}\n", field_access));
                                }
                                "core::integer::u8"
                                | "core::integer::u16"
                                | "core::integer::u32"
                                | "core::integer::u64"
                                | "core::integer::usize" => {
                                    code.push_str(&format!("\tcalldata = append(calldata, cainome.FeltFromUint(uint64({})))\n", field_access));
                                }
                                "core::integer::u128"
                                | "core::integer::i128"
                                | "core::integer::u256" => {
                                    code.push_str(&format!("\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n", field_access));
                                }
                                "core::integer::i8" | "core::integer::i16"
                                | "core::integer::i32" | "core::integer::i64" => {
                                    code.push_str(&format!("\tcalldata = append(calldata, cainome.FeltFromInt(int64({})))\n", field_access));
                                }
                                path if path.starts_with("core::integer::") => {
                                    // Handle any other integer types that might not be explicitly listed
                                    if path.contains("u128")
                                        || path.contains("i128")
                                        || path.contains("u256")
                                    {
                                        code.push_str(&format!("\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n", field_access));
                                    } else {
                                        code.push_str(&format!("\tcalldata = append(calldata, cainome.FeltFromUint(uint64({})))\n", field_access));
                                    }
                                }
                                _ => {
                                    let temp_var = format!("field{}_data", i);
                                    code.push_str(&format!("\tif {}, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}...)\n\t}}\n", temp_var, field_access, error_return, temp_var));
                                }
                            }
                        }
                        Token::Composite(composite) => {
                            // Handle composite types like u256
                            match composite.type_path.as_str() {
                                "core::integer::u256" => {
                                    code.push_str(&format!("\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n", field_access));
                                }
                                _ => {
                                    // For other composite types, use MarshalCairo interface
                                    let temp_var = format!("field{}_data", i);
                                    code.push_str(&format!("\tif {}, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}...)\n\t}}\n", temp_var, field_access, error_return, temp_var));
                                }
                            }
                        }
                        _ => {
                            // For complex types, use MarshalCairo interface
                            let temp_var = format!("field{}_data", i);
                            code.push_str(&format!("\tif {}, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}...)\n\t}}\n", temp_var, field_access, error_return, temp_var));
                        }
                    }
                }
                code
            }
            _ => {
                format!(
                    "\t// TODO: Add serialization for {:?}\n\tcalldata = append(calldata, {})\n",
                    token, param_name
                )
            }
        }
    }

    /// Generate serialization code for basic types with function context for proper error returns
    fn generate_basic_type_serialization_with_context(
        &self,
        token: &Token,
        param_name: &str,
        function: &Function,
    ) -> String {
        let zero_returns = self.generate_zero_returns(function);
        let error_return = if zero_returns.is_empty() {
            "err".to_string()
        } else {
            format!("{}, err", zero_returns)
        };

        match token {
            Token::CoreBasic(core_basic) => {
                match core_basic.type_path.as_str() {
                    "felt" | "core::felt252" => {
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    "core::bool" => {
                        format!("\tif {} {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}} else {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t}}\n", param_name)
                    }
                    "core::integer::u8"
                    | "core::integer::u16"
                    | "core::integer::u32"
                    | "core::integer::u64"
                    | "core::integer::usize" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromUint(uint64({})))\n",
                            param_name
                        )
                    }
                    "core::integer::u128" | "core::integer::i128" | "core::integer::u256" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n",
                            param_name
                        )
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                    | "core::integer::i64" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromInt(int64({})))\n",
                            param_name
                        )
                    }
                    "core::starknet::contract_address::ContractAddress"
                    | "core::starknet::class_hash::ClassHash" => {
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    "core::starknet::eth_address::EthAddress" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBytes({}[:]))\n",
                            param_name
                        )
                    }
                    "core::bytes_31::bytes31" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBytes({}.Bytes()))\n",
                            param_name
                        )
                    }
                    "core::byte_array::ByteArray" => {
                        format!("\tif {}_data, err := cainome.NewCairoByteArray({}).MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                    "[]byte" => {
                        // Handle Go []byte type - assuming it maps to ByteArray
                        format!("\tif {}_data, err := cainome.NewCairoByteArray({}).MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                    "()" => {
                        "".to_string() // Unit type - no serialization needed
                    }
                    _ => {
                        format!("\t// TODO: Add serialization for {}\n\tcalldata = append(calldata, {})\n", core_basic.type_path, param_name)
                    }
                }
            }
            Token::Composite(composite) => {
                // Handle built-in composite types like u256
                match composite.type_path.as_str() {
                    "core::integer::u256" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n",
                            param_name
                        )
                    }
                    "core::byte_array::ByteArray" => {
                        format!("\tif {}_data, err := cainome.NewCairoByteArray({}).MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                    "core::starknet::eth_address::EthAddress" => {
                        format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBytes({}[:]))\n",
                            param_name
                        )
                    }
                    _ => {
                        // For other composite types, try CairoMarshaler interface
                        format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                }
            }
            Token::Array(array) => {
                self.generate_array_serialization_code_with_context(param_name, array, function)
            }
            Token::NonZero(non_zero) => {
                // Handle NonZero types - check if the inner type is a basic type
                match non_zero.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // NonZero<felt> - can be directly appended as *felt.Felt
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    _ => {
                        // Other NonZero types - use CairoMarshaler interface
                        format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
                    }
                }
            }
            Token::Option(option) => {
                // Handle Option types - check if the inner type is a basic type
                match option.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // Option<felt> - handle as **felt.Felt with nil check and direct append
                        format!("\tif {} != nil {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1)) // Some variant\n\t\tcalldata = append(calldata, *{})\n\t}} else {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0)) // None variant\n\t}}\n", param_name, param_name)
                    }
                    Token::Composite(composite) if composite.r#type == CompositeType::Enum => {
                        // Option of enum interface - enum interfaces can be nil directly
                        format!("\tif {} != nil && *{} != nil {{\n\t\t// Some variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t\tif {}_data, err := (*{}).MarshalCairo(); err != nil {{\n\t\t\treturn {}\n\t\t}} else {{\n\t\t\tcalldata = append(calldata, {}_data...)\n\t\t}}\n\t}} else {{\n\t\t// None variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}}\n", param_name, param_name, param_name, param_name, error_return, param_name)
                    }
                    _ => {
                        // Other Option types - handle special cases
                        format!("\tif {} != nil {{\n\t\t// Some variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t\tif {}_data, err := (*{}).MarshalCairo(); err != nil {{\n\t\t\treturn {}\n\t\t}} else {{\n\t\t\tcalldata = append(calldata, {}_data...)\n\t\t}}\n\t}} else {{\n\t\t// None variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}}\n", param_name, param_name, param_name, error_return, param_name)
                    }
                }
            }
            Token::Result(_result) => {
                self.generate_result_serialization_code_with_context(param_name, function)
            }
            Token::Tuple(tuple) => {
                self.generate_tuple_serialization_code_with_context(param_name, tuple, function)
            }
            _ => {
                format!(
                    "\t// TODO: Add serialization for {:?}\n\tcalldata = append(calldata, {})\n",
                    token, param_name
                )
            }
        }
    }

    /// Generate deserialization code for basic types
    fn generate_basic_type_deserialization(&self, token: &Token, go_type: &str) -> String {
        match token {
            Token::CoreBasic(core_basic) => {
                match core_basic.type_path.as_str() {
                    "core::bool" => {
                        "\tresult := cainome.UintFromFelt(response[0]) != 0\n\treturn result, nil\n"
                            .to_string()
                    }
                    "core::integer::u8"
                    | "core::integer::u16"
                    | "core::integer::u32"
                    | "core::integer::u64"
                    | "core::integer::usize" => {
                        format!("\tresult := {}(cainome.UintFromFelt(response[0]))\n\treturn result, nil\n", go_type)
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        "\tresult := cainome.BigIntFromFelt(response[0])\n\treturn result, nil\n"
                            .to_string()
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                    | "core::integer::i64" => {
                        format!(
                            "\tresult := {}(IntFromFelt(response[0]))\n\treturn result, nil\n",
                            go_type
                        )
                    }
                    "core::bytes_31::bytes31" => {
                        "\tresult := BytesFromFelt(response[0])\n\treturn result, nil\n".to_string()
                    }
                    "()" => "\treturn struct{}{}, nil\n".to_string(),
                    "core::starknet::contract_address::ContractAddress"
                    | "core::starknet::class_hash::ClassHash" => {
                        "\tresult := response[0]\n\treturn result, nil\n".to_string()
                    }
                    "core::starknet::eth_address::EthAddress" => {
                        "\tresult := BytesFromFelt(response[0])\n\treturn result, nil\n".to_string()
                    }
                    _ => {
                        format!("\tvar result {}\n\t// TODO: Convert felt to {}\n\t_ = response\n\treturn result, nil\n", go_type, core_basic.type_path)
                    }
                }
            }
            Token::Array(array) => {
                // Handle arrays based on their inner type
                match array.inner.as_ref() {
                    Token::CoreBasic(core_basic)
                        if core_basic.type_path == "felt"
                            || core_basic.type_path == "core::felt252" =>
                    {
                        // []*felt.Felt - deserialize using cainome utilities
                        "\tif len(response) < 1 {\n\t\treturn nil, fmt.Errorf(\"insufficient response data for array\")\n\t}\n\tarrayLength := cainome.UintFromFelt(response[0])\n\tif len(response) < int(1 + arrayLength) {\n\t\treturn nil, fmt.Errorf(\"insufficient response data for array elements\")\n\t}\n\tresult := make([]*felt.Felt, arrayLength)\n\tfor i := uint64(0); i < arrayLength; i++ {\n\t\tresult[i] = response[1+i]\n\t}\n\treturn result, nil\n".to_string()
                    }
                    _ => {
                        // For arrays of complex types, we need to deserialize each element
                        format!("\tvar result {}\n\t// TODO: Implement complex array deserialization for {:?}\n\t_ = response\n\treturn result, nil\n", go_type, array)
                    }
                }
            }
            Token::Option(option) => {
                // Handle Option types with proper Some/None deserialization
                let go_type_inner = go_type.trim_start_matches('*');
                if go_type_inner == "[]*felt.Felt" {
                    // Option<Span<felt>> -> *[]*felt.Felt
                    "\tif len(response) == 0 {\n\t\treturn nil, fmt.Errorf(\"empty response\")\n\t}\n\t// Check Option discriminant\n\tif cainome.UintFromFelt(response[0]) == 0 {\n\t\t// None variant\n\t\treturn nil, nil\n\t} else {\n\t\t// Some variant - deserialize array\n\t\tif len(response) < 2 {\n\t\t\treturn nil, fmt.Errorf(\"insufficient data for Some variant\")\n\t\t}\n\t\tarrayLength := cainome.UintFromFelt(response[1])\n\t\tif len(response) < int(2 + arrayLength) {\n\t\t\treturn nil, fmt.Errorf(\"insufficient data for array elements\")\n\t\t}\n\t\tresult := make([]*felt.Felt, arrayLength)\n\t\tfor i := uint64(0); i < arrayLength; i++ {\n\t\t\tresult[i] = response[2+i]\n\t\t}\n\t\treturn &result, nil\n\t}\n".to_string()
                } else {
                    // Other Option types - need to handle based on inner type
                    match option.inner.as_ref() {
                        Token::Composite(composite)
                            if composite.r#type == CompositeType::Enum
                                || (composite.r#type == CompositeType::Unknown
                                    && composite.type_path.contains("TypedEnum")) =>
                        {
                            // Option<Enum> - deserialize the enum from response[1:]
                            let enum_name = composite.type_name_or_alias();
                            let mut enum_code = String::new();

                            enum_code.push_str("\t\t// Read enum discriminant from response[1]\n");
                            enum_code.push_str(
                                "\t\tdiscriminant := cainome.UintFromFelt(response[1])\n",
                            );
                            enum_code.push_str("\t\tswitch discriminant {\n");

                            // Generate cases for each variant
                            for (index, inner) in composite.inners.iter().enumerate() {
                                let variant_name = inner.name.to_case(Case::Pascal);
                                let variant_type_name = format!("{}{}", enum_name, variant_name);

                                enum_code.push_str(&format!("\t\tcase {}:\n", index));
                                enum_code
                                    .push_str(&format!("\t\t\tvar result {}\n", variant_type_name));
                                enum_code.push_str("\t\t\tif err := result.UnmarshalCairo(response[1:]); err != nil {\n");
                                enum_code.push_str("\t\t\t\treturn nil, fmt.Errorf(\"failed to unmarshal variant: %w\", err)\n");
                                enum_code.push_str("\t\t\t}\n");
                                enum_code.push_str("\t\t\treturn &result, nil\n");
                            }

                            enum_code.push_str("\t\tdefault:\n");
                            enum_code.push_str("\t\t\treturn nil, fmt.Errorf(\"unknown enum discriminant: %d\", discriminant)\n");
                            enum_code.push_str("\t\t}");

                            format!("\tif len(response) == 0 {{\n\t\treturn nil, fmt.Errorf(\"empty response\")\n\t}}\n\t// Check Option discriminant\n\tif cainome.UintFromFelt(response[0]) == 0 {{\n\t\t// None variant\n\t\treturn nil, nil\n\t}} else {{\n\t\t// Some variant - deserialize enum from response[1:]\n\t\tif len(response) < 2 {{\n\t\t\treturn nil, fmt.Errorf(\"insufficient data for Some variant\")\n\t\t}}\n{}\n\t}}\n", enum_code)
                        }
                        Token::CoreBasic(core_basic) => {
                            // Option<basic_type> - deserialize the basic type
                            match core_basic.type_path.as_str() {
                                "core::integer::u64" => {
                                    "\tif len(response) == 0 {\n\t\treturn nil, fmt.Errorf(\"empty response\")\n\t}\n\t// Check Option discriminant\n\tif cainome.UintFromFelt(response[0]) == 0 {\n\t\t// None variant\n\t\treturn nil, nil\n\t} else {\n\t\t// Some variant - extract value\n\t\tif len(response) < 2 {\n\t\t\treturn nil, fmt.Errorf(\"insufficient data for Some variant\")\n\t\t}\n\t\tresult := cainome.UintFromFelt(response[1])\n\t\treturn &result, nil\n\t}\n".to_string()
                                }
                                _ => {
                                    // For other basic types, use the response element directly
                                    "\tif len(response) == 0 {\n\t\treturn nil, fmt.Errorf(\"empty response\")\n\t}\n\t// Check Option discriminant\n\tif cainome.UintFromFelt(response[0]) == 0 {\n\t\t// None variant\n\t\treturn nil, nil\n\t} else {\n\t\t// Some variant - extract value\n\t\tif len(response) < 2 {\n\t\t\treturn nil, fmt.Errorf(\"insufficient data for Some variant\")\n\t\t}\n\t\tresult := response[1]\n\t\treturn &result, nil\n\t}\n".to_string()
                                }
                            }
                        }
                        _ => {
                            // For other types, use the response element directly
                            "\tif len(response) == 0 {\n\t\treturn nil, fmt.Errorf(\"empty response\")\n\t}\n\t// Check Option discriminant\n\tif cainome.UintFromFelt(response[0]) == 0 {\n\t\t// None variant\n\t\treturn nil, nil\n\t} else {\n\t\t// Some variant - extract value\n\t\tif len(response) < 2 {\n\t\t\treturn nil, fmt.Errorf(\"insufficient data for Some variant\")\n\t\t}\n\t\tresult := response[1]\n\t\treturn &result, nil\n\t}\n".to_string()
                        }
                    }
                }
            }
            Token::Tuple(tuple) => self.generate_tuple_response_deserialization(tuple, go_type),
            _ => {
                format!("\tvar result {}\n\t// TODO: Convert felt to {:?}\n\t_ = response\n\treturn result, nil\n", go_type, token)
            }
        }
    }

    /// Generate zero return values for a function based on its outputs (without error)
    fn generate_zero_returns(&self, function: &Function) -> String {
        if function.outputs.is_empty() {
            "".to_string()
        } else if function.outputs.len() == 1 {
            let return_type = self.token_to_go_type(&function.outputs[0]);
            self.generate_zero_value(&return_type)
        } else {
            let zero_returns: Vec<String> = function
                .outputs
                .iter()
                .map(|output| {
                    let return_type = self.token_to_go_type(output);
                    self.generate_zero_value(&return_type)
                })
                .collect();
            zero_returns.join(", ")
        }
    }

    /// Generate zero return values for invoke methods (includes function outputs + nil for txHash)
    fn generate_invoke_zero_returns(&self, function: &Function) -> String {
        let mut returns = Vec::new();

        // Add zero values for function outputs
        for output in &function.outputs {
            let return_type = self.token_to_go_type(output);
            returns.push(self.generate_zero_value(&return_type));
        }

        // Add nil for transaction hash
        returns.push("nil".to_string());

        returns.join(", ")
    }

    /// Generate success return values for invoke methods (includes function outputs + txHash)
    fn generate_invoke_success_returns(&self, function: &Function) -> String {
        let mut returns = Vec::new();

        // For now, return zero values for function outputs (TODO: parse from transaction receipt)
        for output in &function.outputs {
            let return_type = self.token_to_go_type(output);
            returns.push(self.generate_zero_value(&return_type));
        }

        // Add transaction hash
        returns.push("txHash".to_string());

        returns.join(", ")
    }

    /// Generate a zero value for a given Go type
    fn generate_zero_value(&self, go_type: &str) -> String {
        match go_type {
            "*felt.Felt" | "*big.Int" => "nil".to_string(),
            "bool" => "false".to_string(),
            "uint8" | "uint16" | "uint32" | "uint64" | "int8" | "int16" | "int32" | "int64" => {
                "0".to_string()
            }
            "float32" | "float64" => "0.0".to_string(),
            "string" => "\"\"".to_string(),
            s if s.starts_with("*") => "nil".to_string(),
            s if s.starts_with("[]") => "nil".to_string(),
            s if s.starts_with("[") && s.contains("]byte") => {
                // For fixed-size byte arrays like [20]byte
                format!("{}{{}}", s)
            }
            // Check if it's an interface type (currently checking for enum interface pattern)
            s if s.chars().all(|c| c.is_alphanumeric())
                && s.chars().next().unwrap_or('a').is_uppercase()
                && s.ends_with("Enum") =>
            {
                // This is likely an enum interface type - return nil
                "nil".to_string()
            }
            s if s.starts_with("struct {") => {
                // For inline struct types, return the struct literal with the full type
                format!("{}{{}}", s)
            }
            _ => {
                // For named types (structs, enums, etc.), just use the type name as zero value
                format!("{}{{}}", go_type)
            }
        }
    }

    /// Generate serialization code for tuple types with function context for proper error returns
    fn generate_tuple_serialization_code_with_context(
        &self,
        param_name: &str,
        tuple: &cainome_parser::tokens::Tuple,
        function: &Function,
    ) -> String {
        let zero_returns = self.generate_zero_returns(function);
        let error_return = if zero_returns.is_empty() {
            "err".to_string()
        } else {
            format!("{}, err", zero_returns)
        };

        let mut code = String::new();
        code.push_str(&format!(
            "\t// Tuple field {}: serialize each element\n",
            param_name
        ));

        for (i, inner_token) in tuple.inners.iter().enumerate() {
            let field_access = format!("{}.Field{}", param_name, i);
            match inner_token {
                Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                    "felt" | "core::felt252" => {
                        code.push_str(&format!(
                            "\tcalldata = append(calldata, {})\n",
                            field_access
                        ));
                    }
                    "core::bool" => {
                        code.push_str(&format!("\tif {} {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}} else {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t}}\n", field_access));
                    }
                    "core::integer::u8"
                    | "core::integer::u16"
                    | "core::integer::u32"
                    | "core::integer::u64"
                    | "core::integer::usize" => {
                        code.push_str(&format!(
                            "\tcalldata = append(calldata, cainome.FeltFromUint(uint64({})))\n",
                            field_access
                        ));
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        code.push_str(&format!(
                            "\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n",
                            field_access
                        ));
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32"
                    | "core::integer::i64" => {
                        code.push_str(&format!(
                            "\tcalldata = append(calldata, cainome.FeltFromInt(int64({})))\n",
                            field_access
                        ));
                    }
                    "core::starknet::contract_address::ContractAddress"
                    | "core::starknet::class_hash::ClassHash" => {
                        code.push_str(&format!(
                            "\tcalldata = append(calldata, {})\n",
                            field_access
                        ));
                    }
                    _ => {
                        let temp_var = format!("field{}_data", i);
                        code.push_str(&format!("\tif {}, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}...)\n\t}}\n", temp_var, field_access, error_return, temp_var));
                    }
                },
                Token::Composite(composite) => {
                    // Handle composite types like u256
                    match composite.type_path.as_str() {
                        "core::integer::u256" => {
                            code.push_str(&format!(
                                "\tcalldata = append(calldata, cainome.FeltFromBigInt({}))\n",
                                field_access
                            ));
                        }
                        _ => {
                            // For other composite types, use MarshalCairo interface
                            let temp_var = format!("field{}_data", i);
                            code.push_str(&format!("\tif {}, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}...)\n\t}}\n", temp_var, field_access, error_return, temp_var));
                        }
                    }
                }
                _ => {
                    // For complex types, use MarshalCairo interface
                    let temp_var = format!("field{}_data", i);
                    code.push_str(&format!("\tif {}, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}...)\n\t}}\n", temp_var, field_access, error_return, temp_var));
                }
            }
        }

        code
    }

    /// Generate serialization code for array types with function context for proper error returns
    fn generate_array_serialization_code_with_context(
        &self,
        param_name: &str,
        array: &cainome_parser::tokens::Array,
        function: &Function,
    ) -> String {
        let zero_returns = self.generate_zero_returns(function);
        let error_return = if zero_returns.is_empty() {
            "err".to_string()
        } else {
            format!("{}, err", zero_returns)
        };

        match array.inner.as_ref() {
            Token::CoreBasic(core_basic)
                if core_basic.type_path == "felt" || core_basic.type_path == "core::felt252" =>
            {
                // []*felt.Felt - use cainome.CairoFeltArray for serialization
                format!("\tif {}_data, err := cainome.NewCairoFeltArray({}).MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
            }
            _ => {
                // For arrays of structs/other types, iterate and marshal each element
                let mut code = String::new();
                code.push_str(&format!(
                    "\t// Array field {}: serialize length then elements\n",
                    param_name
                ));
                code.push_str(&format!(
                    "\tcalldata = append(calldata, cainome.FeltFromUint(uint64(len({}))))\n",
                    param_name
                ));
                code.push_str(&format!("\tfor _, item := range {} {{\n", param_name));
                code.push_str("\t\tif item_data, err := item.MarshalCairo(); err != nil {\n");
                code.push_str(&format!("\t\t\treturn {}\n", error_return));
                code.push_str("\t\t} else {\n");
                code.push_str("\t\t\tcalldata = append(calldata, item_data...)\n");
                code.push_str("\t\t}\n");
                code.push_str("\t}\n");
                code
            }
        }
    }

    /// Generate serialization code for result types with function context for proper error returns
    fn generate_result_serialization_code_with_context(
        &self,
        param_name: &str,
        function: &Function,
    ) -> String {
        let zero_returns = self.generate_zero_returns(function);
        let error_return = if zero_returns.is_empty() {
            "err".to_string()
        } else {
            format!("{}, err", zero_returns)
        };

        // For Result types, use the MarshalCairo interface
        format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn {}\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, error_return, param_name)
    }

    /// Generate serialization code for option types with invoke context
    fn generate_option_serialization_for_invoke(
        &self,
        param_name: &str,
        option: &cainome_parser::tokens::Option,
        function: &Function,
    ) -> String {
        let zero_returns = self.generate_invoke_zero_returns(function);
        let error_return = format!(
            "{}, fmt.Errorf(\"failed to marshal {}: %w\", err)",
            zero_returns, param_name
        );

        // Handle Option types - for structs, we need special handling
        match option.inner.as_ref() {
            Token::CoreBasic(core_basic)
                if core_basic.type_path == "felt" || core_basic.type_path == "core::felt252" =>
            {
                // Option<felt> - handle as **felt.Felt with nil check and direct append
                format!("\tif {} != nil {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0)) // Some variant\n\t\tcalldata = append(calldata, *{})\n\t}} else {{\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1)) // None variant\n\t}}\n", param_name, param_name)
            }
            Token::Composite(_) => {
                // For Option of struct types (represented as **StructType)
                // We need to check if it's nil (None) or dereference and marshal (Some)
                format!("\tif {} != nil {{\n\t\t// Some variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t\tif {}_data, err := (*{}).MarshalCairo(); err != nil {{\n\t\t\treturn {}\n\t\t}} else {{\n\t\t\tcalldata = append(calldata, {}_data...)\n\t\t}}\n\t}} else {{\n\t\t// None variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}}\n", param_name, param_name, param_name, error_return, param_name)
            }
            _ => {
                // For other Option types, check nil and marshal the inner value
                format!("\tif {} != nil {{\n\t\t// Some variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n\t\tif {}_data, err := (*{}).MarshalCairo(); err != nil {{\n\t\t\treturn {}\n\t\t}} else {{\n\t\t\tcalldata = append(calldata, {}_data...)\n\t\t}}\n\t}} else {{\n\t\t// None variant\n\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n\t}}\n", param_name, param_name, param_name, error_return, param_name)
            }
        }
    }

    /// Generate the body for a state-changing function invoke
    fn generate_invoke_method(&self, function: &Function, receiver_name: &str) -> String {
        let mut method_body = String::new();

        // Setup invoke options
        method_body.push_str("\t// Setup invoke options\n");
        method_body.push_str("\tif opts == nil {\n");
        method_body.push_str("\t\topts = &cainome.InvokeOpts{}\n");
        method_body.push_str("\t}\n\n");

        // Build calldata array
        if function.inputs.is_empty() {
            method_body.push_str("\t// No parameters required\n");
            method_body.push_str("\tcalldata := []*felt.Felt{}\n\n");
        } else {
            method_body.push_str("\t// Serialize parameters to calldata\n");
            method_body.push_str("\tcalldata := []*felt.Felt{}\n");
            for (param_name, param_token) in &function.inputs {
                let param_snake = param_name.to_case(Case::Snake);
                let safe_param_name = self.generate_safe_param_name(&param_snake);

                // Generate serialization code based on parameter type
                match param_token {
                    Token::Option(option) => {
                        // Handle Option types specially
                        match option.inner.as_ref() {
                            Token::Composite(composite)
                                if composite.r#type == CompositeType::Enum =>
                            {
                                // Option of enum interface - enum interfaces can be nil directly
                                method_body
                                    .push_str(&format!("\tif {} != nil {{\n", safe_param_name));
                                method_body.push_str("\t\t// Some variant\n");
                                method_body.push_str(
                                    "\t\tcalldata = append(calldata, cainome.FeltFromUint(0))\n",
                                );
                                method_body.push_str(&format!(
                                    "\t\tif {}_data, err := (*{}).MarshalCairo(); err != nil {{\n",
                                    safe_param_name, safe_param_name
                                ));
                                let zero_returns = self.generate_invoke_zero_returns(function);
                                method_body.push_str(&format!("\t\t\treturn {}, fmt.Errorf(\"failed to marshal {}: %w\", err)\n", zero_returns, safe_param_name));
                                method_body.push_str("\t\t} else {\n");
                                method_body.push_str(&format!(
                                    "\t\t\tcalldata = append(calldata, {}_data...)\n",
                                    safe_param_name
                                ));
                                method_body.push_str("\t\t}\n");
                                method_body.push_str("\t} else {\n");
                                method_body.push_str("\t\t// None variant\n");
                                method_body.push_str(
                                    "\t\tcalldata = append(calldata, cainome.FeltFromUint(1))\n",
                                );
                                method_body.push_str("\t}\n");
                            }
                            _ => {
                                // For other Option types, use the existing function
                                let serialization_code = self
                                    .generate_option_serialization_for_invoke(
                                        &safe_param_name,
                                        option,
                                        function,
                                    );
                                method_body.push_str(&serialization_code);
                            }
                        }
                    }
                    _ if self.is_complex_type(param_token) => {
                        method_body.push_str(&format!(
                            "\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n",
                            safe_param_name, safe_param_name
                        ));
                        // For invoke methods, generate appropriate zero returns based on function outputs
                        let zero_returns = self.generate_invoke_zero_returns(function);
                        method_body.push_str(&format!(
                            "\t\treturn {}, fmt.Errorf(\"failed to marshal {}: %w\", err)\n",
                            zero_returns, safe_param_name
                        ));
                        method_body.push_str("\t} else {\n");
                        method_body.push_str(&format!(
                            "\t\tcalldata = append(calldata, {}_data...)\n",
                            safe_param_name
                        ));
                        method_body.push_str("\t}\n");
                    }
                    _ => {
                        // For basic types, use direct serialization with invoke context
                        let serialization_code = self.generate_basic_type_serialization_for_invoke(
                            param_token,
                            &safe_param_name,
                            function,
                        );
                        method_body.push_str(&serialization_code);
                    }
                }
            }
            method_body.push('\n');
        }

        // Use the provided reference implementation pattern
        method_body.push_str("\t// Build and send invoke transaction using cainome helper\n");
        method_body.push_str(&format!(
            "\ttxHash, err := cainome.BuildAndSendInvokeTxn(ctx, {}.account, {}.contractAddress, utils.GetSelectorFromNameFelt(\"{}\"), calldata, opts)\n",
            receiver_name, receiver_name, function.name
        ));
        method_body.push_str("\tif err != nil {\n");
        let zero_returns = self.generate_invoke_zero_returns(function);
        method_body.push_str(&format!(
            "\t\treturn {}, fmt.Errorf(\"failed to submit invoke transaction: %w\", err)\n",
            zero_returns
        ));
        method_body.push_str("\t}\n\n");

        // Generate return based on function outputs
        let success_returns = self.generate_invoke_success_returns(function);
        method_body.push_str(&format!("\treturn {}, nil\n", success_returns));

        method_body
    }

    /// Generates the main contract struct and constructor
    fn generate_contract(&self, contract_name: &str, functions: &[&Function]) -> String {
        let sanitized_contract_name = self.sanitize_go_identifier(contract_name);
        let struct_name = sanitized_contract_name.to_case(Case::Pascal);
        let mut contract_def = String::new();

        // Generate reader struct for view functions
        contract_def.push_str(&format!("type {}Reader struct {{\n", struct_name));
        contract_def.push_str("\tcontractAddress *felt.Felt\n");
        contract_def.push_str("\tprovider rpc.RpcProvider\n");
        contract_def.push_str("}\n\n");

        // Generate writer struct for invoke functions
        contract_def.push_str(&format!("type {}Writer struct {{\n", struct_name));
        contract_def.push_str("\tcontractAddress *felt.Felt\n");
        contract_def.push_str("\taccount *account.Account\n");
        contract_def.push_str("}\n\n");

        // Generate combined struct that embeds both
        contract_def.push_str(&format!("type {} struct {{\n", struct_name));
        contract_def.push_str(&format!("\t*{}Reader\n", struct_name));
        contract_def.push_str(&format!("\t*{}Writer\n", struct_name));
        contract_def.push_str("}\n\n");

        // Generate reader constructor
        contract_def.push_str(&format!(
            "func New{}Reader(contractAddress *felt.Felt, provider rpc.RpcProvider) *{}Reader {{\n",
            struct_name, struct_name
        ));
        contract_def.push_str(&format!("\treturn &{}Reader {{\n", struct_name));
        contract_def.push_str("\t\tcontractAddress: contractAddress,\n");
        contract_def.push_str("\t\tprovider: provider,\n");
        contract_def.push_str("\t}\n");
        contract_def.push_str("}\n\n");

        // Generate writer constructor
        contract_def.push_str(&format!(
            "func New{}Writer(contractAddress *felt.Felt, account *account.Account) *{}Writer {{\n",
            struct_name, struct_name
        ));
        contract_def.push_str(&format!("\treturn &{}Writer {{\n", struct_name));
        contract_def.push_str("\t\tcontractAddress: contractAddress,\n");
        contract_def.push_str("\t\taccount: account,\n");
        contract_def.push_str("\t}\n");
        contract_def.push_str("}\n\n");

        // Generate combined constructor
        contract_def.push_str(&format!(
            "func New{}(contractAddress *felt.Felt, account *account.Account) *{} {{\n",
            struct_name, struct_name
        ));
        contract_def.push_str(&format!("\treturn &{} {{\n", struct_name));
        contract_def.push_str(&format!(
            "\t\t{}Reader: New{}Reader(contractAddress, account.Provider),\n",
            struct_name, struct_name
        ));
        contract_def.push_str(&format!(
            "\t\t{}Writer: New{}Writer(contractAddress, account),\n",
            struct_name, struct_name
        ));
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
        needs_utils: bool,
        _needs_fmt: bool, // fmt is always needed now
    ) -> String {
        let mut import_lines = vec![
            "\"context\"",
            "\"fmt\"",
            "\"github.com/NethermindEth/juno/core/felt\"",
            "\"github.com/NethermindEth/starknet.go/rpc\"",
            "\"github.com/NethermindEth/starknet.go/account\"",
            "\"github.com/cartridge-gg/cainome\"",
        ];

        if needs_big_int {
            import_lines.push("\"math/big\"");
        }

        if needs_utils {
            import_lines.push("\"github.com/NethermindEth/starknet.go/utils\"");
        }

        // Sort imports for deterministic output
        import_lines.sort();

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
}

#[async_trait]
impl BuiltinPlugin for GolangPlugin {
    async fn generate_code(&self, input: &PluginInput) -> CainomeCliResult<()> {
        tracing::trace!("Golang plugin requested");

        let package_name = &self.options.package_name;

        // Check if any contract needs Result types and generate shared types file
        // No longer need to generate shared types file as all types are in main cainome package

        for contract in &input.contracts {
            let raw_contract_name = contract.name.split("::").last().unwrap_or(&contract.name);
            let sanitized_name = self.sanitize_go_identifier(raw_contract_name);
            let contract_name = sanitized_name.from_case(Case::Snake).to_case(Case::Pascal);

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

            // Sort functions by name for deterministic output
            functions.sort_by(|a, b| a.name.cmp(&b.name));

            // Create sorted vector of composites for deterministic iteration
            let mut sorted_composites: Vec<(&String, &Composite)> =
                composites.iter().map(|(k, &v)| (k, v)).collect();
            sorted_composites.sort_by(|a, b| a.0.cmp(b.0));

            // Find event enums first (from sorted composites)
            let mut event_enums: Vec<&Composite> = Vec::new();
            for (_, composite) in &sorted_composites {
                if composite.r#type == CompositeType::Enum {
                    let enum_name = composite.type_name_or_alias().to_case(Case::Pascal);
                    if enum_name.ends_with("Event") {
                        event_enums.push(composite);
                    }
                }
            }

            // Generate composite types (structs and enums) with contract namespacing
            // Process in deterministic order: first all structs, then all enums
            for (_, composite) in &sorted_composites {
                if composite.r#type == CompositeType::Struct {
                    // Check if this is an event struct that should implement an event interface
                    let original_struct_name = composite.type_name_or_alias().to_case(Case::Pascal);
                    let mut struct_code = self.generate_struct(composite, Some(&contract_name));

                    // Compute the prefixed struct name (same logic as in generate_struct)
                    let mut prefixed_struct_name = original_struct_name.clone();
                    if self.is_event_struct(&original_struct_name) {
                        let sanitized_contract = self.sanitize_go_identifier(&contract_name);
                        let contract_pascal = sanitized_contract.to_case(Case::Pascal);
                        prefixed_struct_name =
                            format!("{}{}", contract_pascal, original_struct_name);
                    }

                    // Find matching event enum and generate interface implementations
                    for event_enum in &event_enums {
                        // Check if this struct is one of the event variants
                        for inner in &event_enum.inners {
                            let variant_name = inner.name.to_case(Case::Pascal);

                            // Check if the original struct name matches the event variant
                            let matches = if original_struct_name
                                .ends_with(&format!("Event{}", variant_name))
                            {
                                true
                            } else if original_struct_name.ends_with(&variant_name) {
                                // Also check if it just ends with the variant name for backward compatibility
                                true
                            } else {
                                // Check if the variant's type matches this struct
                                // Extract the type name from the variant's type path
                                if let Token::Composite(variant_composite) = &inner.token {
                                    let variant_type_name = variant_composite
                                        .type_name_or_alias()
                                        .to_case(Case::Pascal);
                                    original_struct_name.ends_with(&variant_type_name)
                                } else {
                                    false
                                }
                            };

                            if matches {
                                struct_code.push_str(&self.generate_event_struct_implementation(
                                    &prefixed_struct_name, // Use the prefixed name here
                                    event_enum,
                                    &contract_name,
                                ));
                                break;
                            }
                        }
                    }

                    generated_code_temp.push_str(&struct_code);
                    generated_code_temp.push('\n');
                }
            }

            // Now process enums in deterministic order
            for (_, composite) in &sorted_composites {
                if composite.r#type == CompositeType::Enum {
                    generated_code_temp
                        .push_str(&self.generate_enum(composite, Some(&contract_name)));
                    generated_code_temp.push('\n');
                }
            }

            // Generate contract struct and methods
            generated_code_temp.push_str(&self.generate_contract(&contract_name, &functions));

            // Check if generated code actually uses big.Int and utils
            let needs_big_int = generated_code_temp.contains("*big.Int");
            let needs_utils = generated_code_temp.contains("utils.GetSelectorFromNameFelt");

            let mut generated_code =
                self.generate_package_header(package_name, needs_big_int, needs_utils, false);

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
    use crate::contract::{ContractData, ContractParser, ContractParserConfig};
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

        // Test felt252 mappings
        test_felt_mappings(&test_output_dir).await;

        // Test integer mappings
        test_integer_mappings(&test_output_dir).await;
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

    /// Test sanitization of contract names with special characters
    #[test]
    fn test_contract_name_sanitization() {
        let plugin = GolangPlugin::new(crate::args::GolangPluginOptions {
            package_name: "test".to_string(),
        });

        // Test various special character scenarios
        assert_eq!(
            plugin.sanitize_go_identifier("controller.v_1.0.9"),
            "controller_v_1_0_9"
        );
        assert_eq!(plugin.sanitize_go_identifier("my-contract"), "my_contract");
        assert_eq!(
            plugin.sanitize_go_identifier("contract@v2"),
            "contract_at_v2"
        );
        assert_eq!(
            plugin.sanitize_go_identifier("contract+plus"),
            "contract_plus_plus"
        );
        assert_eq!(
            plugin.sanitize_go_identifier("contract with spaces"),
            "contract_with_spaces"
        );
        assert_eq!(plugin.sanitize_go_identifier("123contract"), "_123contract");
        assert_eq!(
            plugin.sanitize_go_identifier("contract...name"),
            "contract_name"
        );
        assert_eq!(plugin.sanitize_go_identifier("_contract_"), "_contract");
        assert_eq!(
            plugin.sanitize_go_identifier("contract!@#$%^&*()"),
            "contract_at"
        );
        assert_eq!(plugin.sanitize_go_identifier("valid_name"), "valid_name");
    }
}
