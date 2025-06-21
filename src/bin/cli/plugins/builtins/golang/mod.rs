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
                let inner_type = self.token_to_go_type(&option.inner);
                format!("*{}", inner_type) // Use pointer for optional types
            }
            Token::Result(result) => {
                // Generate a Result type that can be unpacked into (value, error) pattern
                let ok_type = self.token_to_go_type(&result.inner);
                let err_type = self.token_to_go_type(&result.error);
                format!("Result[{}, {}]", ok_type, err_type)
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
                } else {
                    // Use pointer to struct type for pass-by-reference parameters
                    let type_name = composite.type_name_or_alias().to_case(Case::Pascal);
                    format!("*{}", type_name)
                }
            }
            Token::Option(option) => {
                let inner_type = self.token_to_go_param_type(&option.inner);
                format!("*{}", inner_type) // Use pointer for optional types
            }
            Token::Result(result) => {
                // Generate a Result type that can be unpacked into (value, error) pattern
                let ok_type = self.token_to_go_param_type(&result.inner);
                let err_type = self.token_to_go_param_type(&result.error);
                format!("Result[{}, {}]", ok_type, err_type)
            }
            Token::NonZero(non_zero) => self.token_to_go_param_type(&non_zero.inner),
            Token::Function(_) => "func".to_string(),
        }
    }

    /// Generates Go struct definition for a Cairo composite type
    fn generate_struct(&self, composite: &Composite) -> String {
        let struct_name = composite.type_name_or_alias().to_case(Case::Pascal);
        let mut struct_def = format!("type {} struct {{\n", struct_name);

        for inner in &composite.inners {
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
    fn generate_enum(&self, composite: &Composite) -> String {
        let enum_name = composite.type_name_or_alias().to_case(Case::Pascal);

        // Check if this is an event enum (ends with "Event")
        if enum_name.ends_with("Event") {
            return self.generate_event_enum(composite);
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
                    let data_type = self.token_to_go_type(&inner.token);
                    enum_def.push_str(&format!(
                        "func {}(value {}) {} {{\n\treturn {} {{\n\t\tVariant: \"{}\",\n\t\tValue: value,\n\t}}\n}}\n\n",
                        constructor_name, data_type, enum_name, enum_name, inner.name
                    ));
                }
                _ => {}
            }
        }

        // Generate CairoMarshaler implementation for the enum
        enum_def.push_str(&self.generate_enum_cairo_marshaler(&enum_name, composite));

        enum_def
    }

    /// Generates Go event interface for Cairo event enum types (idiomatic Go approach)
    fn generate_event_enum(&self, composite: &Composite) -> String {
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

    /// Generates CairoMarshaler implementation for a struct
    fn generate_struct_cairo_marshaler(&self, struct_name: &str, composite: &Composite) -> String {
        let mut marshaler = String::new();

        // Generate MarshalCairo method
        marshaler.push_str(&format!("// MarshalCairo serializes {} to Cairo felt array\n", struct_name));
        marshaler.push_str(&format!("func (s *{}) MarshalCairo() ([]*felt.Felt, error) {{\n", struct_name));
        marshaler.push_str("\tvar result []*felt.Felt\n\n");

        // Serialize each field
        for inner in &composite.inners {
            let field_name = inner.name.to_case(Case::Pascal);
            marshaler.push_str(&self.generate_field_marshal_code(&field_name, &inner.token));
        }

        marshaler.push_str("\treturn result, nil\n");
        marshaler.push_str("}\n\n");

        // Generate UnmarshalCairo method
        marshaler.push_str(&format!("// UnmarshalCairo deserializes {} from Cairo felt array\n", struct_name));
        marshaler.push_str(&format!("func (s *{}) UnmarshalCairo(data []*felt.Felt) error {{\n", struct_name));
        
        // Only declare offset if we have fields to unmarshal
        if !composite.inners.is_empty() {
            marshaler.push_str("\toffset := 0\n\n");
        }

        // Deserialize each field
        for inner in &composite.inners {
            let field_name = inner.name.to_case(Case::Pascal);
            marshaler.push_str(&self.generate_field_unmarshal_code(&field_name, &inner.token));
        }

        marshaler.push_str("\treturn nil\n");
        marshaler.push_str("}\n\n");

        // Generate CairoSize method
        marshaler.push_str(&format!("// CairoSize returns the serialized size for {}\n", struct_name));
        marshaler.push_str(&format!("func (s *{}) CairoSize() int {{\n", struct_name));
        marshaler.push_str("\treturn -1 // Dynamic size\n");
        marshaler.push_str("}\n\n");

        marshaler
    }

    /// Generates marshal code for array fields
    fn generate_array_marshal_code(&self, field_name: &str, inner_token: &Token) -> String {
        let mut code = format!("\t// Array field {}: serialize length then elements\n", field_name);
        code.push_str(&format!("\tresult = append(result, FeltFromUint(uint64(len(s.{}))))\n", field_name));
        code.push_str(&format!("\tfor _, item := range s.{} {{\n", field_name));
        
        match inner_token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    code.push_str("\t\tresult = append(result, item)\n");
                }
                "core::bool" => {
                    code.push_str("\t\tresult = append(result, FeltFromBool(item))\n");
                }
                "core::integer::u8" | "core::integer::u16" | "core::integer::u32" | "core::integer::u64" | "core::integer::usize" => {
                    code.push_str("\t\tresult = append(result, FeltFromUint(uint64(item)))\n");
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str("\t\tresult = append(result, FeltFromBigInt(item))\n");
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32" | "core::integer::i64" => {
                    code.push_str("\t\tresult = append(result, FeltFromUint(uint64(item)))\n");
                }
                "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
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
                            code.push_str("\t\tresult = append(result, FeltFromBigInt(item))\n");
                        }
                        _ => {
                            code.push_str("\t\t// TODO: Handle unknown builtin composite type in array\n");
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
        let mut code = format!("\t// Array field {}: read length then elements\n", field_name);
        code.push_str(&format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for array length of {}\")\n\t}}\n", field_name));
        code.push_str(&format!("\tlength{} := UintFromFelt(data[offset])\n", field_name));
        code.push_str("\toffset++\n");
        
        // Get the Go type for the array element
        let element_type = self.token_to_go_type(inner_token);
        code.push_str(&format!("\ts.{} = make([]{}, length{})\n", field_name, element_type, field_name));
        code.push_str(&format!("\tfor i := uint64(0); i < length{}; i++ {{\n", field_name));
        
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
                    code.push_str(&format!("\t\ts.{}[i] = BoolFromFelt(data[offset])\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = uint8(UintFromFelt(data[offset]))\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = uint16(UintFromFelt(data[offset]))\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = uint32(UintFromFelt(data[offset]))\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u64" | "core::integer::usize" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = UintFromFelt(data[offset])\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = BigIntFromFelt(data[offset])\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = int8(UintFromFelt(data[offset]))\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = int16(UintFromFelt(data[offset]))\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = int32(UintFromFelt(data[offset]))\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i64" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str(&format!("\t\ts.{}[i] = int64(UintFromFelt(data[offset]))\n", field_name));
                    code.push_str("\t\toffset++\n");
                }
                "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
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
                            code.push_str("\t\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for array element %d of {}\", i)\n", field_name));
                            code.push_str("\t\t}\n");
                            code.push_str(&format!("\t\ts.{}[i] = BigIntFromFelt(data[offset])\n", field_name));
                            code.push_str("\t\toffset++\n");
                        }
                        _ => {
                            code.push_str("\t\t// TODO: Handle unknown builtin composite type in array unmarshal\n");
                        }
                    }
                } else {
                    // Custom struct/enum - use CairoMarshaler
                    code.push_str(&format!("\t\tvar item {}\n", element_type));
                    code.push_str("\t\tif err := item.UnmarshalCairo(data[offset:]); err != nil {\n");
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
    fn generate_tuple_marshal_code(&self, field_name: &str, tuple: &cainome_parser::tokens::Tuple) -> String {
        let mut code = format!("\t// Tuple field {}: marshal each sub-field\n", field_name);
        
        for (index, inner_token) in tuple.inners.iter().enumerate() {
            let field_access = format!("s.{}.Field{}", field_name, index);
            match inner_token {
                Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                    "felt" | "core::felt252" => {
                        code.push_str(&format!("\tresult = append(result, {})\n", field_access));
                    }
                    "core::bool" => {
                        code.push_str(&format!("\tresult = append(result, FeltFromBool({}))\n", field_access));
                    }
                    "core::integer::u8" | "core::integer::u16" | "core::integer::u32" | "core::integer::u64" | "core::integer::usize" => {
                        code.push_str(&format!("\tresult = append(result, FeltFromUint(uint64({})))\n", field_access));
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        code.push_str(&format!("\tresult = append(result, FeltFromBigInt({}))\n", field_access));
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32" | "core::integer::i64" => {
                        code.push_str(&format!("\tresult = append(result, FeltFromUint(uint64({})))\n", field_access));
                    }
                    "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                        code.push_str(&format!("\tresult = append(result, {})\n", field_access));
                    }
                    _ => {
                        code.push_str(&format!("\t// TODO: Handle unknown basic type in tuple field {}\n", index));
                    }
                },
                Token::Composite(composite) => {
                    if composite.is_builtin() {
                        match composite.type_path_no_generic().as_str() {
                            "core::integer::u256" => {
                                code.push_str(&format!("\tresult = append(result, FeltFromBigInt({}))\n", field_access));
                            }
                            _ => {
                                code.push_str(&format!("\t// TODO: Handle unknown builtin composite type in tuple field {}\n", index));
                            }
                        }
                    } else {
                        // Custom struct/enum - use CairoMarshaler
                        code.push_str(&format!("\tif fieldData, err := {}.MarshalCairo(); err != nil {{\n", field_access));
                        code.push_str("\t\treturn nil, err\n");
                        code.push_str("\t} else {\n");
                        code.push_str("\t\tresult = append(result, fieldData...)\n");
                        code.push_str("\t}\n");
                    }
                }
                Token::Array(_) => {
                    code.push_str(&format!("\t// TODO: Handle array type in tuple field {}\n", index));
                }
                _ => {
                    code.push_str(&format!("\t// TODO: Handle unknown token type in tuple field {}\n", index));
                }
            }
        }
        
        code
    }

    /// Generates unmarshal code for tuple fields
    fn generate_tuple_unmarshal_code(&self, field_name: &str, tuple: &cainome_parser::tokens::Tuple) -> String {
        let mut code = format!("\t// Tuple field {}: unmarshal each sub-field\n", field_name);
        
        for (index, inner_token) in tuple.inners.iter().enumerate() {
            let field_access = format!("s.{}.Field{}", field_name, index);
            match inner_token {
                Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
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
                        code.push_str(&format!("\t{} = BoolFromFelt(data[offset])\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::u8" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = uint8(UintFromFelt(data[offset]))\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::u16" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = uint16(UintFromFelt(data[offset]))\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::u32" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = uint32(UintFromFelt(data[offset]))\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::u64" | "core::integer::usize" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = UintFromFelt(data[offset])\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = BigIntFromFelt(data[offset])\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::i8" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = int8(UintFromFelt(data[offset]))\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::i16" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = int16(UintFromFelt(data[offset]))\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::i32" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = int32(UintFromFelt(data[offset]))\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::integer::i64" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = int64(UintFromFelt(data[offset]))\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                        code.push_str("\tif offset >= len(data) {\n");
                        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                        code.push_str("\t}\n");
                        code.push_str(&format!("\t{} = data[offset]\n", field_access));
                        code.push_str("\toffset++\n");
                    }
                    _ => {
                        code.push_str(&format!("\t// TODO: Handle unknown basic type in tuple field {} element {}\n", field_name, index));
                    }
                },
                Token::Composite(composite) => {
                    if composite.is_builtin() {
                        match composite.type_path_no_generic().as_str() {
                            "core::integer::u256" => {
                                code.push_str("\tif offset >= len(data) {\n");
                                code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for tuple field {} element {}\")\n", field_name, index));
                                code.push_str("\t}\n");
                                code.push_str(&format!("\t{} = BigIntFromFelt(data[offset])\n", field_access));
                                code.push_str("\toffset++\n");
                            }
                            _ => {
                                code.push_str(&format!("\t// TODO: Handle unknown builtin composite type in tuple field {} element {}\n", field_name, index));
                            }
                        }
                    } else {
                        // Custom struct/enum - use CairoMarshaler
                        code.push_str(&format!("\tif err := {}.UnmarshalCairo(data[offset:]); err != nil {{\n", field_access));
                        code.push_str("\t\treturn err\n");
                        code.push_str("\t}\n");
                        code.push_str("\t// Calculate consumed felts to update offset\n");
                        code.push_str(&format!("\tif itemData, err := {}.MarshalCairo(); err != nil {{\n", field_access));
                        code.push_str("\t\treturn err\n");
                        code.push_str("\t} else {\n");
                        code.push_str("\t\toffset += len(itemData)\n");
                        code.push_str("\t}\n");
                    }
                }
                Token::Array(_) => {
                    code.push_str(&format!("\t// TODO: Handle array type in tuple field {} element {}\n", field_name, index));
                }
                _ => {
                    code.push_str(&format!("\t// TODO: Handle unknown token type in tuple field {} element {}\n", field_name, index));
                }
            }
        }
        
        code.push_str("\n");
        code
    }

    /// Generates marshal code for Option fields
    fn generate_option_marshal_code(&self, field_name: &str, inner_token: &Token) -> String {
        let mut code = format!("\t// Option field {}: check for nil and marshal accordingly\n", field_name);
        code.push_str(&format!("\tif s.{} != nil {{\n", field_name));
        code.push_str("\t\t// Some variant: discriminant 0 + value\n");
        code.push_str("\t\tresult = append(result, FeltFromUint(0))\n");
        
        // Marshal the inner value based on its type
        match inner_token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    code.push_str(&format!("\t\tresult = append(result, *s.{})\n", field_name));
                }
                "core::bool" => {
                    code.push_str(&format!("\t\tresult = append(result, FeltFromBool(*s.{}))\n", field_name));
                }
                "core::integer::u8" | "core::integer::u16" | "core::integer::u32" | "core::integer::u64" | "core::integer::usize" => {
                    code.push_str(&format!("\t\tresult = append(result, FeltFromUint(uint64(*s.{})))\n", field_name));
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str(&format!("\t\tresult = append(result, FeltFromBigInt(*s.{}))\n", field_name));
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32" | "core::integer::i64" => {
                    code.push_str(&format!("\t\tresult = append(result, FeltFromUint(uint64(*s.{})))\n", field_name));
                }
                "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                    code.push_str(&format!("\t\tresult = append(result, *s.{})\n", field_name));
                }
                _ => {
                    code.push_str(&format!("\t\t// TODO: Handle unknown basic type in Option field {}\n", field_name));
                }
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            code.push_str(&format!("\t\tresult = append(result, FeltFromBigInt(*s.{}))\n", field_name));
                        }
                        _ => {
                            code.push_str(&format!("\t\t// TODO: Handle unknown builtin composite type in Option field {}\n", field_name));
                        }
                    }
                } else {
                    // Custom struct/enum - use CairoMarshaler
                    code.push_str(&format!("\t\tif fieldData, err := s.{}.MarshalCairo(); err != nil {{\n", field_name));
                    code.push_str("\t\t\treturn nil, err\n");
                    code.push_str("\t\t} else {\n");
                    code.push_str("\t\t\tresult = append(result, fieldData...)\n");
                    code.push_str("\t\t}\n");
                }
            }
            _ => {
                code.push_str(&format!("\t\t// TODO: Handle unknown token type in Option field {}\n", field_name));
            }
        }
        
        code.push_str("\t} else {\n");
        code.push_str("\t\t// None variant: discriminant 1 (no additional data)\n");
        code.push_str("\t\tresult = append(result, FeltFromUint(1))\n");
        code.push_str("\t}\n");
        code
    }

    /// Generates unmarshal code for Option fields
    fn generate_option_unmarshal_code(&self, field_name: &str, inner_token: &Token) -> String {
        let mut code = format!("\t// Option field {}: read discriminant then value if Some\n", field_name);
        code.push_str("\tif offset >= len(data) {\n");
        code.push_str(&format!("\t\treturn fmt.Errorf(\"insufficient data for Option field {} discriminant\")\n", field_name));
        code.push_str("\t}\n");
        code.push_str("\tdiscriminant := UintFromFelt(data[offset])\n");
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
                    code.push_str("\t\tvalue = BoolFromFelt(data[offset])\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = uint8(UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = uint16(UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = uint32(UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u64" | "core::integer::usize" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = UintFromFelt(data[offset])\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::u128" | "core::integer::i128" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = BigIntFromFelt(data[offset])\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i8" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int8(UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i16" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int16(UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i32" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int32(UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::integer::i64" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = int64(UintFromFelt(data[offset]))\n");
                    code.push_str("\t\toffset++\n");
                }
                "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                    code.push_str("\t\tif offset >= len(data) {\n");
                    code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                    code.push_str("\t\t}\n");
                    code.push_str("\t\tvalue = data[offset]\n");
                    code.push_str("\t\toffset++\n");
                }
                _ => {
                    code.push_str(&format!("\t\t// TODO: Handle unknown basic type in Option field {} unmarshal\n", field_name));
                }
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::integer::u256" => {
                            code.push_str("\t\tif offset >= len(data) {\n");
                            code.push_str(&format!("\t\t\treturn fmt.Errorf(\"insufficient data for Option field {} value\")\n", field_name));
                            code.push_str("\t\t}\n");
                            code.push_str("\t\tvalue = BigIntFromFelt(data[offset])\n");
                            code.push_str("\t\toffset++\n");
                        }
                        _ => {
                            code.push_str(&format!("\t\t// TODO: Handle unknown builtin composite type in Option field {} unmarshal\n", field_name));
                        }
                    }
                } else {
                    // Custom struct/enum - use CairoMarshaler
                    code.push_str("\t\tif err := value.UnmarshalCairo(data[offset:]); err != nil {\n");
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
                code.push_str(&format!("\t\t// TODO: Handle unknown token type in Option field {} unmarshal\n", field_name));
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
    fn generate_result_marshal_code(&self, field_name: &str, _ok_token: &Token, _err_token: &Token) -> String {
        format!("\t// Result field {}: marshal using Cairo Result pattern\n\tif fieldData, err := s.{}.MarshalCairo(); err != nil {{\n\t\treturn nil, err\n\t}} else {{\n\t\tresult = append(result, fieldData...)\n\t}}\n", field_name, field_name)
    }

    /// Generates unmarshal code for Result fields
    fn generate_result_unmarshal_code(&self, field_name: &str, _ok_token: &Token, _err_token: &Token) -> String {
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
                    format!("\tresult = append(result, FeltFromBool(s.{}))\n", field_name)
                }
                "core::integer::u8" | "core::integer::u16" | "core::integer::u32" | "core::integer::u64" | "core::integer::usize" => {
                    format!("\tresult = append(result, FeltFromUint(uint64(s.{})))\n", field_name)
                }
                "core::integer::u128" | "core::integer::i128" => {
                    format!("\tresult = append(result, FeltFromBigInt(s.{}))\n", field_name)
                }
                "core::integer::i8" | "core::integer::i16" | "core::integer::i32" | "core::integer::i64" => {
                    format!("\tresult = append(result, FeltFromUint(uint64(s.{})))\n", field_name)
                }
                "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                    format!("\tresult = append(result, s.{})\n", field_name)
                }
                "core::bytes_31::bytes31" => {
                    format!("\t// TODO: Handle bytes31 conversion for field {}\n", field_name)
                }
                _ => format!("\t// TODO: Handle unknown core basic type for field {}\n", field_name),
            },
            Token::Array(array) => {
                self.generate_array_marshal_code(field_name, &array.inner)
            }
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    match composite.type_path_no_generic().as_str() {
                        "core::byte_array::ByteArray" => {
                            format!("\t// TODO: Handle ByteArray serialization for field {}\n", field_name)
                        }
                        "core::integer::u256" => {
                            format!("\tresult = append(result, FeltFromBigInt(s.{}))\n", field_name)
                        }
                        _ => format!("\t// TODO: Handle builtin composite {} for field {}\n", composite.type_path_no_generic(), field_name),
                    }
                } else {
                    format!("\t// Struct field {}: marshal using CairoMarshaler\n\tif fieldData, err := s.{}.MarshalCairo(); err != nil {{\n\t\treturn nil, err\n\t}} else {{\n\t\tresult = append(result, fieldData...)\n\t}}\n", field_name, field_name)
                }
            }
            Token::Option(option) => {
                self.generate_option_marshal_code(field_name, &option.inner)
            }
            Token::Result(result) => {
                self.generate_result_marshal_code(field_name, &result.inner, &result.error)
            }
            Token::NonZero(non_zero) => {
                // NonZero types are just wrappers, marshal the inner type directly
                self.generate_field_marshal_code(field_name, &non_zero.inner)
            }
            Token::Tuple(tuple) => {
                self.generate_tuple_marshal_code(field_name, tuple)
            }
            _ => format!("\t// TODO: Handle unknown token type for field {}\n", field_name),
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
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = BoolFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u8" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = uint8(UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u16" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = uint16(UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u32" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = uint32(UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u64" | "core::integer::usize" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = UintFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::u128" | "core::integer::i128" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = BigIntFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i8" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int8(UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i16" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int16(UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i32" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int32(UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::integer::i64" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = int64(UintFromFelt(data[offset]))\n\toffset++\n\n", field_name, field_name)
                }
                "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                    format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = data[offset]\n\toffset++\n\n", field_name, field_name)
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
                            format!("\tif offset >= len(data) {{\n\t\treturn fmt.Errorf(\"insufficient data for field {}\")\n\t}}\n\ts.{} = BigIntFromFelt(data[offset])\n\toffset++\n\n", field_name, field_name)
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

    /// Generates CairoMarshaler implementation for an enum
    fn generate_enum_cairo_marshaler(&self, enum_name: &str, composite: &Composite) -> String {
        let mut marshaler = String::new();

        // Generate MarshalCairo method
        marshaler.push_str(&format!("// MarshalCairo serializes {} to Cairo felt array\n", enum_name));
        marshaler.push_str(&format!("func (e *{}) MarshalCairo() ([]*felt.Felt, error) {{\n", enum_name));
        marshaler.push_str("\tvar result []*felt.Felt\n\n");

        // Switch on variant to serialize discriminant + data
        marshaler.push_str("\tswitch e.Variant {\n");
        for (index, inner) in composite.inners.iter().enumerate() {
            let variant_name = inner.name.clone();
            marshaler.push_str(&format!("\tcase \"{}\":\n", variant_name));
            marshaler.push_str(&format!("\t\t// Discriminant for variant {}\n", variant_name));
            marshaler.push_str(&format!("\t\tresult = append(result, FeltFromUint({}))\n", index));

            // Handle variant data based on CompositeInnerKind
            match inner.kind {
                CompositeInnerKind::NotUsed => {
                    // Unit variant (no additional data)
                    marshaler.push_str("\t\t// Unit variant - no additional data\n");
                }
                CompositeInnerKind::Data => {
                    // Data variant - serialize the value
                    marshaler.push_str(&self.generate_enum_variant_marshal_code(&inner.token));
                }
                _ => {
                    marshaler.push_str("\t\t// TODO: Handle other CompositeInnerKind variants\n");
                }
            }
        }
        marshaler.push_str("\tdefault:\n");
        marshaler.push_str(&format!("\t\treturn nil, fmt.Errorf(\"unknown variant: %s\", e.Variant)\n"));
        marshaler.push_str("\t}\n\n");
        marshaler.push_str("\treturn result, nil\n");
        marshaler.push_str("}\n\n");

        // Generate UnmarshalCairo method
        marshaler.push_str(&format!("// UnmarshalCairo deserializes {} from Cairo felt array\n", enum_name));
        marshaler.push_str(&format!("func (e *{}) UnmarshalCairo(data []*felt.Felt) error {{\n", enum_name));
        marshaler.push_str("\tif len(data) == 0 {\n");
        marshaler.push_str("\t\treturn fmt.Errorf(\"insufficient data for enum discriminant\")\n");
        marshaler.push_str("\t}\n\n");
        marshaler.push_str("\tdiscriminant := UintFromFelt(data[0])\n");
        marshaler.push_str("\toffset := 1\n\n");

        // Switch on discriminant to deserialize
        marshaler.push_str("\tswitch discriminant {\n");
        for (index, inner) in composite.inners.iter().enumerate() {
            let variant_name = inner.name.clone();
            marshaler.push_str(&format!("\tcase {}:\n", index));
            marshaler.push_str(&format!("\t\te.Variant = \"{}\"\n", variant_name));

            match inner.kind {
                CompositeInnerKind::NotUsed => {
                    // Unit variant (no additional data)
                    marshaler.push_str("\t\te.Value = nil\n");
                }
                CompositeInnerKind::Data => {
                    // Data variant - deserialize the value
                    marshaler.push_str(&self.generate_enum_variant_unmarshal_code(&inner.token));
                }
                _ => {
                    marshaler.push_str("\t\t// TODO: Handle other CompositeInnerKind variants\n");
                }
            }
        }
        marshaler.push_str("\tdefault:\n");
        marshaler.push_str(&format!("\t\treturn fmt.Errorf(\"unknown discriminant: %d\", discriminant)\n"));
        marshaler.push_str("\t}\n\n");
        
        // Check if offset is used - if all variants are unit variants, suppress unused warning
        let has_data_variants = composite.inners.iter().any(|inner| matches!(inner.kind, CompositeInnerKind::Data));
        if !has_data_variants {
            marshaler.push_str("\t_ = offset // Suppress unused variable warning for unit-only enums\n");
        }
        
        marshaler.push_str("\treturn nil\n");
        marshaler.push_str("}\n\n");

        // Generate CairoSize method
        marshaler.push_str(&format!("// CairoSize returns the serialized size for {}\n", enum_name));
        marshaler.push_str(&format!("func (e *{}) CairoSize() int {{\n", enum_name));
        marshaler.push_str("\treturn -1 // Dynamic size\n");
        marshaler.push_str("}\n\n");

        marshaler
    }

    /// Generates marshal code for enum variant data
    fn generate_enum_variant_marshal_code(&self, token: &Token) -> String {
        match token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    "\t\tif value, ok := e.Value.(*felt.Felt); ok {\n\t\t\tresult = append(result, value)\n\t\t} else {\n\t\t\treturn nil, fmt.Errorf(\"invalid value type for felt variant\")\n\t\t}\n".to_string()
                }
                "core::bool" => {
                    "\t\tif value, ok := e.Value.(bool); ok {\n\t\t\tresult = append(result, FeltFromBool(value))\n\t\t} else {\n\t\t\treturn nil, fmt.Errorf(\"invalid value type for bool variant\")\n\t\t}\n".to_string()
                }
                "core::integer::u8" | "core::integer::u16" | "core::integer::u32" | "core::integer::u64" | "core::integer::usize" => {
                    "\t\tif value, ok := e.Value.(uint64); ok {\n\t\t\tresult = append(result, FeltFromUint(value))\n\t\t} else {\n\t\t\treturn nil, fmt.Errorf(\"invalid value type for uint variant\")\n\t\t}\n".to_string()
                }
                "core::integer::u128" | "core::integer::i128" => {
                    "\t\tif value, ok := e.Value.(*big.Int); ok {\n\t\t\tresult = append(result, FeltFromBigInt(value))\n\t\t} else {\n\t\t\treturn nil, fmt.Errorf(\"invalid value type for big.Int variant\")\n\t\t}\n".to_string()
                }
                _ => "\t\t// TODO: Handle unknown core basic type for enum variant\n".to_string(),
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    "\t\t// TODO: Handle builtin composite type for enum variant\n".to_string()
                } else {
                    "\t\tif value, ok := e.Value.(CairoMarshaler); ok {\n\t\t\tif valueData, err := value.MarshalCairo(); err != nil {\n\t\t\t\treturn nil, err\n\t\t\t} else {\n\t\t\t\tresult = append(result, valueData...)\n\t\t\t}\n\t\t} else {\n\t\t\treturn nil, fmt.Errorf(\"invalid value type for struct variant\")\n\t\t}\n".to_string()
                }
            }
            Token::Array(_) => {
                "\t\t// TODO: Handle array type for enum variant\n".to_string()
            }
            Token::Tuple(_) => {
                "\t\t// TODO: Handle tuple type for enum variant\n".to_string()
            }
            _ => "\t\t// TODO: Handle unknown token type for enum variant\n".to_string(),
        }
    }

    /// Generates unmarshal code for enum variant data
    fn generate_enum_variant_unmarshal_code(&self, token: &Token) -> String {
        match token {
            Token::CoreBasic(core_basic) => match core_basic.type_path.as_str() {
                "felt" | "core::felt252" => {
                    "\t\tif offset >= len(data) {\n\t\t\treturn fmt.Errorf(\"insufficient data for felt variant\")\n\t\t}\n\t\te.Value = data[offset]\n".to_string()
                }
                "core::bool" => {
                    "\t\tif offset >= len(data) {\n\t\t\treturn fmt.Errorf(\"insufficient data for bool variant\")\n\t\t}\n\t\te.Value = BoolFromFelt(data[offset])\n".to_string()
                }
                "core::integer::u8" => {
                    "\t\tif offset >= len(data) {\n\t\t\treturn fmt.Errorf(\"insufficient data for u8 variant\")\n\t\t}\n\t\te.Value = uint8(UintFromFelt(data[offset]))\n".to_string()
                }
                "core::integer::u16" => {
                    "\t\tif offset >= len(data) {\n\t\t\treturn fmt.Errorf(\"insufficient data for u16 variant\")\n\t\t}\n\t\te.Value = uint16(UintFromFelt(data[offset]))\n".to_string()
                }
                "core::integer::u32" => {
                    "\t\tif offset >= len(data) {\n\t\t\treturn fmt.Errorf(\"insufficient data for u32 variant\")\n\t\t}\n\t\te.Value = uint32(UintFromFelt(data[offset]))\n".to_string()
                }
                "core::integer::u64" | "core::integer::usize" => {
                    "\t\tif offset >= len(data) {\n\t\t\treturn fmt.Errorf(\"insufficient data for u64 variant\")\n\t\t}\n\t\te.Value = UintFromFelt(data[offset])\n".to_string()
                }
                "core::integer::u128" | "core::integer::i128" => {
                    "\t\tif offset >= len(data) {\n\t\t\treturn fmt.Errorf(\"insufficient data for big.Int variant\")\n\t\t}\n\t\te.Value = BigIntFromFelt(data[offset])\n".to_string()
                }
                _ => "\t\t// TODO: Handle unknown core basic type for enum variant unmarshal\n\t\t_ = offset // Suppress unused variable warning\n".to_string(),
            },
            Token::Composite(composite) => {
                if composite.is_builtin() {
                    "\t\t// TODO: Handle builtin composite type for enum variant unmarshal\n\t\t_ = offset // Suppress unused variable warning\n".to_string()
                } else {
                    let type_name = composite.type_name_or_alias().to_case(Case::Pascal);
                    format!("\t\tvar value {}\n\t\tif err := value.UnmarshalCairo(data[offset:]); err != nil {{\n\t\t\treturn err\n\t\t}}\n\t\te.Value = value\n", type_name)
                }
            }
            Token::Array(_) => {
                "\t\t// TODO: Handle array type for enum variant unmarshal\n\t\t_ = offset // Suppress unused variable warning\n".to_string()
            }
            Token::Tuple(_) => {
                "\t\t// TODO: Handle tuple type for enum variant unmarshal\n\t\t_ = offset // Suppress unused variable warning\n".to_string()
            }
            _ => "\t\t// TODO: Handle unknown token type for enum variant unmarshal\n\t\t_ = offset // Suppress unused variable warning\n".to_string(),
        }
    }

    /// Generates Go function definition for a Cairo contract function
    fn generate_function(&self, function: &Function, contract_name: &str) -> String {
        let func_name = function.name.to_case(Case::Pascal);
        let receiver_name = contract_name.to_case(Case::Snake);

        let is_view = function.state_mutability == StateMutability::View;

        // Generate parameters
        let mut params = Vec::new();

        // Add context as first parameter
        params.push("ctx context.Context".to_string());

        // Add contract function parameters
        for (param_name, param_token) in &function.inputs {
            let go_type = self.token_to_go_param_type(param_token);
            let param_snake = param_name.to_case(Case::Snake);
            let safe_param_name = self.generate_safe_param_name(&param_snake);
            params.push(format!("{} {}", safe_param_name, go_type));
        }

        // Add opts parameter for view functions
        if is_view {
            params.push("opts *CallOpts".to_string());
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

        let mut func_def = format!(
            "func ({} *{}) {}({}) {} {{\n",
            receiver_name,
            contract_name.to_case(Case::Pascal),
            func_name,
            params.join(", "),
            return_str
        );

        if is_view {
            // Generate call method for view functions
            func_def.push_str(&self.generate_call_method(function, &receiver_name));
        } else {
            // Generate invoke method for state-changing functions
            func_def.push_str(&self.generate_invoke_method(function, &receiver_name));
        }

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
        method_body.push_str("\t\topts = &CallOpts{}\n");
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
                if self.is_complex_type(param_token) {
                    method_body.push_str(&format!(
                        "\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n",
                        safe_param_name, safe_param_name
                    ));
                    let zero_returns = self.generate_zero_returns(function);
                    let first_return = if zero_returns.is_empty() {
                        format!("fmt.Errorf(\"failed to marshal {}: %w\", err)", safe_param_name)
                    } else {
                        format!("{}, fmt.Errorf(\"failed to marshal {}: %w\", err)", zero_returns.split(", ").next().unwrap_or(""), safe_param_name)
                    };
                    method_body.push_str(&format!(
                        "\t\treturn {}\n",
                        first_return
                    ));
                    method_body.push_str("\t} else {\n");
                    method_body.push_str(&format!("\t\tcalldata = append(calldata, {}_data...)\n", safe_param_name));
                    method_body.push_str("\t}\n");
                } else {
                    // For basic types, use direct serialization
                    let serialization_code = self.generate_basic_type_serialization(param_token, &safe_param_name);
                    method_body.push_str(&serialization_code);
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
                let deserialization_code = self.generate_basic_type_deserialization(&function.outputs[0], &return_type);
                method_body.push_str(&deserialization_code);
            }
        } else {
            method_body.push_str("\t// TODO: Deserialize response to proper types\n");
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
            method_body
                .push_str("\t// For now, return zero values - proper deserialization needed\n");
            let return_values: Vec<String> = function
                .outputs
                .iter()
                .enumerate()
                .map(|(i, output)| {
                    let return_type = self.token_to_go_type(output);
                    if return_type == "*felt.Felt" && i < 1 {
                        "response[0]".to_string()
                    } else {
                        self.generate_zero_value(&return_type)
                    }
                })
                .collect();
            method_body.push_str("\t_ = response // TODO: deserialize response\n");
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
        matches!(token, Token::Composite(composite) if !composite.is_builtin())
    }

    /// Generate serialization code for basic types
    fn generate_basic_type_serialization(&self, token: &Token, param_name: &str) -> String {
        match token {
            Token::CoreBasic(core_basic) => {
                match core_basic.type_path.as_str() {
                    "felt" | "core::felt252" => {
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    "core::bool" => {
                        format!("\tif {} {{\n\t\tcalldata = append(calldata, FeltFromUint(1))\n\t}} else {{\n\t\tcalldata = append(calldata, FeltFromUint(0))\n\t}}\n", param_name)
                    }
                    "core::integer::u8" | "core::integer::u16" | "core::integer::u32" | "core::integer::u64" | "core::integer::usize" => {
                        format!("\tcalldata = append(calldata, FeltFromUint(uint64({})))\n", param_name)
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        format!("\tcalldata = append(calldata, FeltFromBigInt({}))\n", param_name)
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32" | "core::integer::i64" => {
                        format!("\tcalldata = append(calldata, FeltFromInt(int64({})))\n", param_name)
                    }
                    "core::starknet::contract_address::ContractAddress" | "core::starknet::class_hash::ClassHash" => {
                        format!("\tcalldata = append(calldata, {})\n", param_name)
                    }
                    "core::bytes_31::bytes31" => {
                        format!("\tcalldata = append(calldata, FeltFromBytes({}.Bytes()))\n", param_name)
                    }
                    "()" => {
                        "".to_string() // Unit type - no serialization needed
                    }
                    _ => {
                        format!("\t// TODO: Add serialization for {}\n\tcalldata = append(calldata, {})\n", core_basic.type_path, param_name)
                    }
                }
            }
            Token::Array(_) => {
                format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn fmt.Errorf(\"failed to marshal {}: %w\", err)\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, param_name, param_name)
            }
            Token::Option(_) | Token::Result(_) | Token::NonZero(_) => {
                format!("\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n\t\treturn fmt.Errorf(\"failed to marshal {}: %w\", err)\n\t}} else {{\n\t\tcalldata = append(calldata, {}_data...)\n\t}}\n", param_name, param_name, param_name, param_name)
            }
            _ => {
                format!("\t// TODO: Add serialization for {:?}\n\tcalldata = append(calldata, {})\n", token, param_name)
            }
        }
    }

    /// Generate deserialization code for basic types
    fn generate_basic_type_deserialization(&self, token: &Token, go_type: &str) -> String {
        match token {
            Token::CoreBasic(core_basic) => {
                match core_basic.type_path.as_str() {
                    "core::bool" => {
                        "\tresult := UintFromFelt(response[0]) != 0\n\treturn result, nil\n".to_string()
                    }
                    "core::integer::u8" | "core::integer::u16" | "core::integer::u32" | "core::integer::u64" | "core::integer::usize" => {
                        format!("\tresult := {}(UintFromFelt(response[0]))\n\treturn result, nil\n", go_type)
                    }
                    "core::integer::u128" | "core::integer::i128" => {
                        "\tresult := BigIntFromFelt(response[0])\n\treturn result, nil\n".to_string()
                    }
                    "core::integer::i8" | "core::integer::i16" | "core::integer::i32" | "core::integer::i64" => {
                        format!("\tresult := {}(IntFromFelt(response[0]))\n\treturn result, nil\n", go_type)
                    }
                    "core::bytes_31::bytes31" => {
                        "\tresult := BytesFromFelt(response[0])\n\treturn result, nil\n".to_string()
                    }
                    "()" => {
                        "\treturn struct{}{}, nil\n".to_string()
                    }
                    _ => {
                        format!("\tvar result {}\n\t// TODO: Convert felt to {}\n\t_ = response\n\treturn result, nil\n", go_type, core_basic.type_path)
                    }
                }
            }
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

    /// Generate the body for a state-changing function invoke
    fn generate_invoke_method(&self, function: &Function, _receiver_name: &str) -> String {
        let mut method_body = String::new();

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
                if self.is_complex_type(param_token) {
                    method_body.push_str(&format!(
                        "\tif {}_data, err := {}.MarshalCairo(); err != nil {{\n",
                        safe_param_name, safe_param_name
                    ));
                    let zero_returns = self.generate_zero_returns(function);
                    let first_return = if zero_returns.is_empty() {
                        format!("fmt.Errorf(\"failed to marshal {}: %w\", err)", safe_param_name)
                    } else {
                        format!("{}, fmt.Errorf(\"failed to marshal {}: %w\", err)", zero_returns.split(", ").next().unwrap_or(""), safe_param_name)
                    };
                    method_body.push_str(&format!(
                        "\t\treturn {}\n",
                        first_return
                    ));
                    method_body.push_str("\t} else {\n");
                    method_body.push_str(&format!("\t\tcalldata = append(calldata, {}_data...)\n", safe_param_name));
                    method_body.push_str("\t}\n");
                } else {
                    // For basic types, use direct serialization
                    let serialization_code = self.generate_basic_type_serialization(param_token, &safe_param_name);
                    method_body.push_str(&serialization_code);
                }
            }
            method_body.push('\n');
        }

        method_body.push_str("\t// TODO: Implement invoke transaction\n");
        method_body
            .push_str("\t// This requires account/signer setup for transaction submission\n");
        method_body.push_str("\t_ = calldata\n");

        // Generate proper return statement based on function outputs
        if function.outputs.is_empty() {
            method_body.push_str("\treturn fmt.Errorf(\"invoke methods require account setup - not yet implemented\")\n");
        } else if function.outputs.len() == 1 {
            let return_type = self.token_to_go_type(&function.outputs[0]);
            let zero_value = self.generate_zero_value(&return_type);
            method_body.push_str(&format!("\treturn {}, fmt.Errorf(\"invoke methods require account setup - not yet implemented\")\n", zero_value));
        } else {
            // Multiple return values
            let zero_returns: Vec<String> = function
                .outputs
                .iter()
                .map(|output| {
                    let return_type = self.token_to_go_type(output);
                    self.generate_zero_value(&return_type)
                })
                .collect();
            method_body.push_str(&format!("\treturn {}, fmt.Errorf(\"invoke methods require account setup - not yet implemented\")\n", zero_returns.join(", ")));
        }

        method_body
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
        needs_utils: bool,
        _needs_fmt: bool, // fmt is always needed now
    ) -> String {
        let mut import_lines = vec![
            "\"context\"",
            "\"fmt\"",
            "\"github.com/NethermindEth/juno/core/felt\"",
            "\"github.com/NethermindEth/starknet.go/rpc\"",
        ];

        if needs_big_int {
            import_lines.insert(2, "\"math/big\"");
        }

        if needs_utils {
            import_lines.push("\"github.com/NethermindEth/starknet.go/utils\"");
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
	"math/big"
	"github.com/NethermindEth/juno/core/felt"
	"github.com/NethermindEth/starknet.go/rpc"
)

"#,
            package_name
        );

        // Add CallOpts type definition for optional call parameters
        types_content.push_str(
            r#"// CallOpts contains options for contract view calls
type CallOpts struct {
	BlockID *rpc.BlockID // Optional block ID (defaults to "latest" if nil)
}

// CallOption defines a function type for setting call options
type CallOption func(*CallOpts)

// WithBlockID sets the block ID for the call
func WithBlockID(blockID rpc.BlockID) CallOption {
	return func(opts *CallOpts) {
		opts.BlockID = &blockID
	}
}

// NewCallOpts creates a new CallOpts with optional configurations
func NewCallOpts(options ...CallOption) *CallOpts {
	opts := &CallOpts{}
	for _, option := range options {
		option(opts)
	}
	return opts
}

// CairoMarshaler interface for types that can be serialized/deserialized to/from Cairo felt arrays
type CairoMarshaler interface {
	// MarshalCairo serializes the type to a Cairo felt array
	MarshalCairo() ([]*felt.Felt, error)
	
	// UnmarshalCairo deserializes the type from a Cairo felt array
	UnmarshalCairo(data []*felt.Felt) error
}

// CairoSerde provides static size information for Cairo serialization
type CairoSerde interface {
	CairoMarshaler
	
	// CairoSize returns the serialized size in felts, or -1 for dynamic size
	CairoSize() int
}

// Result type for handling Cairo Result types with idiomatic Go error handling
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

// Helper functions for Cairo serialization

// FeltFromUint converts a uint64 to *felt.Felt
func FeltFromUint(value uint64) *felt.Felt {
	return new(felt.Felt).SetUint64(value)
}

// FeltFromBigInt converts a *big.Int to *felt.Felt
func FeltFromBigInt(value *big.Int) *felt.Felt {
	if value == nil {
		return new(felt.Felt)
	}
	result := new(felt.Felt)
	result.SetBytes(value.Bytes())
	return result
}

// FeltFromBool converts a bool to *felt.Felt (0 for false, 1 for true)
func FeltFromBool(value bool) *felt.Felt {
	if value {
		return new(felt.Felt).SetUint64(1)
	}
	return new(felt.Felt)
}

// UintFromFelt converts *felt.Felt to uint64
func UintFromFelt(f *felt.Felt) uint64 {
	if f == nil {
		return 0
	}
	return f.Uint64()
}

// BigIntFromFelt converts *felt.Felt to *big.Int
func BigIntFromFelt(f *felt.Felt) *big.Int {
	if f == nil {
		return big.NewInt(0)
	}
	return f.BigInt(big.NewInt(0))
}

// BoolFromFelt converts *felt.Felt to bool (0 is false, anything else is true)
func BoolFromFelt(f *felt.Felt) bool {
	if f == nil {
		return false
	}
	return !f.IsZero()
}

// CairoSerializeArray serializes an array with length prefix
func CairoSerializeArray(items []CairoMarshaler) ([]*felt.Felt, error) {
	result := []*felt.Felt{FeltFromUint(uint64(len(items)))}
	for _, item := range items {
		data, err := item.MarshalCairo()
		if err != nil {
			return nil, err
		}
		result = append(result, data...)
	}
	return result, nil
}

// CairoDeserializeArray deserializes an array with length prefix
func CairoDeserializeArray(data []*felt.Felt, offset int, createItem func() CairoMarshaler) ([]CairoMarshaler, int, error) {
	if len(data) <= offset {
		return nil, offset, fmt.Errorf("insufficient data for array length")
	}
	
	length := UintFromFelt(data[offset])
	offset++
	
	result := make([]CairoMarshaler, length)
	for i := uint64(0); i < length; i++ {
		item := createItem()
		if err := item.UnmarshalCairo(data[offset:]); err != nil {
			return nil, offset, err
		}
		
		// Calculate how many felts this item consumed
		itemData, err := item.MarshalCairo()
		if err != nil {
			return nil, offset, err
		}
		offset += len(itemData)
		result[i] = item
	}
	
	return result, offset, nil
}

// Basic type wrapper implementations for CairoMarshaler interface

// CairoFelt wraps *felt.Felt to implement CairoMarshaler
type CairoFelt struct {
	Value *felt.Felt
}

func (f *CairoFelt) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{f.Value}, nil
}

func (f *CairoFelt) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for felt")
	}
	f.Value = data[0]
	return nil
}

func (f *CairoFelt) CairoSize() int {
	return 1
}

// CairoUint64 wraps uint64 to implement CairoMarshaler
type CairoUint64 struct {
	Value uint64
}

func (u *CairoUint64) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(u.Value)}, nil
}

func (u *CairoUint64) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint64")
	}
	u.Value = UintFromFelt(data[0])
	return nil
}

func (u *CairoUint64) CairoSize() int {
	return 1
}

// CairoUint32 wraps uint32 to implement CairoMarshaler
type CairoUint32 struct {
	Value uint32
}

func (u *CairoUint32) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(uint64(u.Value))}, nil
}

func (u *CairoUint32) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint32")
	}
	u.Value = uint32(UintFromFelt(data[0]))
	return nil
}

func (u *CairoUint32) CairoSize() int {
	return 1
}

// CairoUint16 wraps uint16 to implement CairoMarshaler
type CairoUint16 struct {
	Value uint16
}

func (u *CairoUint16) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(uint64(u.Value))}, nil
}

func (u *CairoUint16) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint16")
	}
	u.Value = uint16(UintFromFelt(data[0]))
	return nil
}

func (u *CairoUint16) CairoSize() int {
	return 1
}

// CairoUint8 wraps uint8 to implement CairoMarshaler
type CairoUint8 struct {
	Value uint8
}

func (u *CairoUint8) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromUint(uint64(u.Value))}, nil
}

func (u *CairoUint8) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for uint8")
	}
	u.Value = uint8(UintFromFelt(data[0]))
	return nil
}

func (u *CairoUint8) CairoSize() int {
	return 1
}

// CairoBigInt wraps *big.Int to implement CairoMarshaler  
type CairoBigInt struct {
	Value *big.Int
}

func (b *CairoBigInt) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromBigInt(b.Value)}, nil
}

func (b *CairoBigInt) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for big.Int")
	}
	b.Value = BigIntFromFelt(data[0])
	return nil
}

func (b *CairoBigInt) CairoSize() int {
	return 1
}

// CairoBool wraps bool to implement CairoMarshaler
type CairoBool struct {
	Value bool
}

func (b *CairoBool) MarshalCairo() ([]*felt.Felt, error) {
	return []*felt.Felt{FeltFromBool(b.Value)}, nil
}

func (b *CairoBool) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for bool")
	}
	b.Value = BoolFromFelt(data[0])
	return nil
}

func (b *CairoBool) CairoSize() int {
	return 1
}

// CairoFeltArray wraps []*felt.Felt to implement CairoMarshaler with length prefix
type CairoFeltArray struct {
	Value []*felt.Felt
}

func (a *CairoFeltArray) MarshalCairo() ([]*felt.Felt, error) {
	result := []*felt.Felt{FeltFromUint(uint64(len(a.Value)))}
	result = append(result, a.Value...)
	return result, nil
}

func (a *CairoFeltArray) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for array length")
	}
	length := UintFromFelt(data[0])
	if len(data) < int(length)+1 {
		return fmt.Errorf("insufficient data for array elements")
	}
	a.Value = data[1:length+1]
	return nil
}

func (a *CairoFeltArray) CairoSize() int {
	return -1 // Dynamic size
}

// CairoMarshaler implementation for Result[T, E]
func (r Result[T, E]) MarshalCairo() ([]*felt.Felt, error) {
	var result []*felt.Felt
	
	if r.IsOk {
		// Ok variant: discriminant 0 + value
		result = append(result, FeltFromUint(0))
		// Try to marshal Ok value if it implements CairoMarshaler
		if marshaler, ok := any(r.Ok).(CairoMarshaler); ok {
			data, err := marshaler.MarshalCairo()
			if err != nil {
				return nil, err
			}
			result = append(result, data...)
		} else {
			// For basic types, try to convert directly
			switch v := any(r.Ok).(type) {
			case *felt.Felt:
				result = append(result, v)
			case uint64:
				result = append(result, FeltFromUint(v))
			case *big.Int:
				result = append(result, FeltFromBigInt(v))
			case bool:
				result = append(result, FeltFromBool(v))
			default:
				return nil, fmt.Errorf("unsupported Ok type for Result marshaling")
			}
		}
	} else {
		// Err variant: discriminant 1 + error value
		result = append(result, FeltFromUint(1))
		// Try to marshal Err value if it implements CairoMarshaler
		if marshaler, ok := any(r.Err).(CairoMarshaler); ok {
			data, err := marshaler.MarshalCairo()
			if err != nil {
				return nil, err
			}
			result = append(result, data...)
		} else {
			// For basic types, try to convert directly
			switch v := any(r.Err).(type) {
			case *felt.Felt:
				result = append(result, v)
			case uint64:
				result = append(result, FeltFromUint(v))
			case *big.Int:
				result = append(result, FeltFromBigInt(v))
			case bool:
				result = append(result, FeltFromBool(v))
			default:
				return nil, fmt.Errorf("unsupported Err type for Result marshaling")
			}
		}
	}
	
	return result, nil
}

func (r *Result[T, E]) UnmarshalCairo(data []*felt.Felt) error {
	if len(data) == 0 {
		return fmt.Errorf("insufficient data for Result discriminant")
	}
	
	discriminant := UintFromFelt(data[0])
	offset := 1
	
	if discriminant == 0 {
		// Ok variant
		r.IsOk = true
		// Try to unmarshal Ok value if it implements CairoMarshaler
		if unmarshaler, ok := any(&r.Ok).(CairoMarshaler); ok {
			if err := unmarshaler.UnmarshalCairo(data[offset:]); err != nil {
				return err
			}
		} else {
			// For basic types, try to convert directly
			if offset >= len(data) {
				return fmt.Errorf("insufficient data for Result Ok value")
			}
			switch any(r.Ok).(type) {
			case *felt.Felt:
				r.Ok = any(data[offset]).(T)
			case uint64:
				r.Ok = any(UintFromFelt(data[offset])).(T)
			case *big.Int:
				r.Ok = any(BigIntFromFelt(data[offset])).(T)
			case bool:
				r.Ok = any(BoolFromFelt(data[offset])).(T)
			default:
				return fmt.Errorf("unsupported Ok type for Result unmarshaling")
			}
		}
	} else {
		// Err variant
		r.IsOk = false
		// Try to unmarshal Err value if it implements CairoMarshaler
		if unmarshaler, ok := any(&r.Err).(CairoMarshaler); ok {
			if err := unmarshaler.UnmarshalCairo(data[offset:]); err != nil {
				return err
			}
		} else {
			// For basic types, try to convert directly
			if offset >= len(data) {
				return fmt.Errorf("insufficient data for Result Err value")
			}
			switch any(r.Err).(type) {
			case *felt.Felt:
				r.Err = any(data[offset]).(E)
			case uint64:
				r.Err = any(UintFromFelt(data[offset])).(E)
			case *big.Int:
				r.Err = any(BigIntFromFelt(data[offset])).(E)
			case bool:
				r.Err = any(BoolFromFelt(data[offset])).(E)
			default:
				return fmt.Errorf("unsupported Err type for Result unmarshaling")
			}
		}
	}
	
	return nil
}

func (r Result[T, E]) CairoSize() int {
	return -1 // Dynamic size
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
                        let mut struct_code = self.generate_struct(composite);

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
                        generated_code_temp.push_str(&self.generate_enum(composite));
                        generated_code_temp.push('\n');
                    }
                    _ => {}
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
            go_content
                .contains("GetBytes31(ctx context.Context, opts *CallOpts) ([31]byte, error)"),
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
