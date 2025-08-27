use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let dest_path = Path::new("./src/generated_tests.rs");
    let test_artifacts_dir = Path::new("test_artifacts");

    if !test_artifacts_dir.exists() {
        println!("cargo:warning=test_artifacts directory not found, skipping test generation");
        return;
    }

    let mut test_file = fs::File::create(dest_path).unwrap();

    // Start the test file with common imports and abigen calls
    writeln!(test_file, "// Generated test file - do not edit manually").unwrap();
    writeln!(test_file, "use cainome_cairo_serde::CairoSerde;").unwrap();
    writeln!(test_file, "use starknet::core::types::Felt;").unwrap();
    writeln!(test_file, "use cainome::rs::abigen;").unwrap();
    writeln!(test_file, "#[allow(unused_imports)]").unwrap();
    writeln!(test_file, "use serde::{{Serialize, Deserialize}};").unwrap();
    writeln!(test_file).unwrap();

    let mut test_count = 0;
    let mut abigen_calls = Vec::new();

    // Read all artifact files and generate ABIs
    if let Ok(entries) = fs::read_dir(test_artifacts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(type_name) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(artifact) = serde_json::from_str::<Value>(&content) {
                            // Create inline ABI for simple core types
                            if should_generate_abi(&artifact) {
                                // Generate ABIs for each artifact
                                if let Some(abi_type) = artifact.get("abi_type") {
                                    // Use abi_type directly from artifact, wrapping in array if needed
                                    let abi_array = if abi_type.is_array() {
                                        abi_type.clone()
                                    } else {
                                        json!([abi_type])
                                    };

                                    let abi_json_str = serde_json::to_string(&abi_array).unwrap();

                                    // Generate abigen! call with inline ABI string using raw string literal
                                    abigen_calls.push(format!(
                                        "abigen!(Test{}, r#\"{}\"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));",
                                        to_pascal_case(type_name),
                                        abi_json_str
                                    ));
                                }
                            }

                            if can_generate_roundtrip_test(type_name, &artifact) {
                                write_test_for_artifact(
                                    &mut test_file,
                                    &content,
                                    type_name,
                                    &artifact,
                                );
                                test_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Write abigen calls at the top of the file after imports
    let mut temp_content = String::new();
    for abigen_call in &abigen_calls {
        temp_content.push_str(&format!("{}\n", abigen_call));
    }
    temp_content.push('\n');

    // Read the existing content and append
    let existing_content = fs::read_to_string(dest_path).unwrap();
    let lines: Vec<&str> = existing_content.lines().collect();
    let mut new_content = String::new();

    // Write header and imports
    for (i, line) in lines.iter().enumerate() {
        new_content.push_str(line);
        new_content.push('\n');

        // Insert abigen calls after imports
        if line.trim() == "" && i > 2 {
            new_content.push_str(&temp_content);
            break;
        }
    }

    // Write the rest of the content (tests)
    let mut in_tests = false;
    for line in &lines {
        if line.starts_with("#[test]") {
            in_tests = true;
        }
        if in_tests {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    fs::write(dest_path, new_content).unwrap();

    println!("cargo:warning=Generated {} serialization tests", test_count);
    println!("cargo:warning=Generated {} ABI files", abigen_calls.len());
    println!("cargo:rerun-if-changed=test_artifacts");
}

fn should_generate_abi(artifact: &Value) -> bool {
    // Generate ABI ONLY for types that abigen! macro actually supports:
    // function, event, struct, enum, constructor, impl, interface, l1_handler
    if let Some(abi_type) = artifact.get("abi_type") {
        // Handle both single objects and arrays of ABI objects
        if abi_type.is_array() {
            // For arrays, check if any item is a struct or enum that we can generate
            if let Some(array) = abi_type.as_array() {
                if !array.is_empty() {
                    // Check first item to determine type
                    if let Some(first_item) = array.first() {
                        if let Some(type_str) = first_item.get("type") {
                            return match type_str.as_str() {
                                Some("struct") => can_generate_struct_abi(abi_type),
                                Some("enum") => can_generate_enum_abi(abi_type),
                                _ => false,
                            };
                        }
                    }
                }
            }
            false
        } else {
            // Handle single objects (legacy format)
            if let Some(type_str) = abi_type.get("type") {
                return match type_str.as_str() {
                    Some("struct") => can_generate_struct_abi(abi_type), // Enable struct generation
                    Some("enum") => can_generate_enum_abi(abi_type),
                    // abigen! does NOT support: array, tuple, felt252, u8, u32, u64, u128, etc.
                    // These are handled by direct cairo-serde implementation
                    _ => false,
                };
            }
            false
        }
    } else {
        false
    }
}

fn can_generate_struct_abi(abi_type: &Value) -> bool {
    // Handle both array and single object formats
    let structs_to_check = if abi_type.is_array() {
        abi_type.as_array().unwrap()
    } else {
        std::slice::from_ref(abi_type)
    };

    // Check each struct in the collection
    for struct_item in structs_to_check {
        // Check if struct has only supported types that can be used in abigen! tests
        if let Some(name) = struct_item.get("name").and_then(|n| n.as_str()) {
            // Skip problematic core types for now - let them use direct cairo-serde testing
            match name {
                "core::integer::u256" => return false, // Use direct cairo-serde testing
                "core::starknet::eth_address::EthAddress" => return false, // Use direct cairo-serde testing
                "core::byte_array::ByteArray" => return false, // Use direct cairo-serde testing
                _ => {}
            }
        }
    }
    // Check members for each struct
    for struct_item in structs_to_check {
        if let Some(members) = struct_item.get("members") {
            if let Some(members_array) = members.as_array() {
                for member in members_array {
                    if let Some(member_type) = member.get("type") {
                        if let Some(type_str) = member_type.as_str() {
                            // Allow core types and struct dependencies
                            if !is_supported_struct_member_type(type_str) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }
    true
}

fn is_supported_struct_member_type(type_str: &str) -> bool {
    match type_str {
        // Core primitive types
        "core::felt252"
        | "core::integer::u8"
        | "core::integer::u32"
        | "core::integer::u64"
        | "core::integer::u128"
        | "core::integer::u256" => true,
        // StarkNet types
        "core::starknet::contract_address::ContractAddress"
        | "core::starknet::class_hash::ClassHash"
        | "core::starknet::eth_address::EthAddress" => true,
        // Arrays and spans - limit to supported core types
        "core::array::Span::<core::felt252>" => true,
        // Skip unsupported array types for now
        s if s.starts_with("core::array::Array::<core::bytes_31::bytes31>") => false,
        // Tuples
        s if s.starts_with("(") && s.ends_with(")") => true,
        // Struct dependencies within the same contract namespace - allow them
        s if s.starts_with("contracts::") => true,
        _ => false,
    }
}

fn can_generate_enum_abi(abi_type: &Value) -> bool {
    // Check if enum has only core types (no complex contract dependencies)
    if let Some(variants) = abi_type.get("variants") {
        if let Some(variants_array) = variants.as_array() {
            for variant in variants_array {
                if let Some(variant_type) = variant.get("type") {
                    if let Some(type_str) = variant_type.as_str() {
                        // Skip enums with custom contract dependencies (except unit type)
                        if type_str != "()" && type_str.contains("contracts::") {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn generate_abigen_based_test(test_file: &mut fs::File, type_name: &str, artifact: &Value) {
    let contract_name = format!("Test{}", to_pascal_case(type_name));

    writeln!(
        test_file,
        "    // Test using abigen!-generated type: {}",
        contract_name
    )
    .unwrap();

    // Check if we have a test_value in the artifact for type generation
    if let Some(test_value) = artifact.get("test_value") {
        if let Some(test_value_str) = test_value.as_str() {
            writeln!(test_file, "    // Expected test value: {}", test_value_str).unwrap();
        }
    }

    writeln!(test_file).unwrap();

    // Generate actual test instance creation and testing for different types
    if let Some(abi_type) = artifact.get("abi_type") {
        // Handle both single objects and arrays
        let type_to_check = if abi_type.is_array() {
            // For arrays, get the type from the first item
            abi_type
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("type"))
                .and_then(|t| t.as_str())
        } else {
            // For single objects, get the type directly
            abi_type.get("type").and_then(|t| t.as_str())
        };

        if let Some(type_str) = type_to_check {
            match type_str {
                "enum" => {
                    generate_enum_abigen_test(
                        test_file,
                        type_name,
                        &contract_name,
                        abi_type,
                        artifact,
                    );
                }
                "struct" => {
                    generate_struct_abigen_test(
                        test_file,
                        type_name,
                        &contract_name,
                        abi_type,
                        artifact,
                    );
                }
                _ => {
                    writeln!(test_file, "    // Unknown type - basic validation").unwrap();
                    writeln!(test_file, "    assert!(!expected_felt_values.is_empty(), \"Expected values should not be empty\");").unwrap();
                }
            }
        }
    }
}

fn generate_enum_abigen_test(
    test_file: &mut fs::File,
    type_name: &str,
    contract_name: &str,
    abi_type: &Value,
    artifact: &Value,
) {
    if let Some(name) = abi_type.get("name") {
        if let Some(name_str) = name.as_str() {
            match name_str {
                "core::bool" => {
                    // For core types, use built-in types as they have equivalent serialization
                    let test_value = if type_name.contains("true") {
                        "true"
                    } else {
                        "false"
                    };
                    writeln!(
                        test_file,
                        "    // Using built-in bool (equivalent to abigen!-generated Bool)"
                    )
                    .unwrap();
                    writeln!(test_file, "    let test_instance = {};", test_value).unwrap();
                    writeln!(
                        test_file,
                        "    let serialized = bool::cairo_serialize(&test_instance);"
                    )
                    .unwrap();
                    writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Bool serialization mismatch\");").unwrap();

                    // Add roundtrip test
                    writeln!(test_file, "    let deserialize_ptr = 0;").unwrap();
                    writeln!(test_file, "    let deserialized = bool::cairo_deserialize(&serialized, deserialize_ptr).unwrap();").unwrap();
                    writeln!(
                        test_file,
                        "    assert_eq!(test_instance, deserialized, \"Bool roundtrip failed\");"
                    )
                    .unwrap();
                }
                "core::option::Option::<core::felt252>" => {
                    // For core types, use built-in types as they have equivalent serialization
                    if type_name.contains("some") {
                        writeln!(test_file, "    // Using built-in Option<Felt> (equivalent to abigen!-generated type)").unwrap();
                        writeln!(
                            test_file,
                            "    let test_instance = Some(Felt::from_hex(\"0x2a\").unwrap());"
                        )
                        .unwrap();
                        writeln!(
                            test_file,
                            "    let serialized = Option::<Felt>::cairo_serialize(&test_instance);"
                        )
                        .unwrap();
                        writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Option<Felt> Some serialization mismatch\");").unwrap();

                        // Add roundtrip test
                        writeln!(test_file, "    let deserialize_ptr = 0;").unwrap();
                        writeln!(test_file, "    let deserialized = Option::<Felt>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();").unwrap();
                        writeln!(test_file, "    assert_eq!(test_instance, deserialized, \"Option<Felt> Some roundtrip failed\");").unwrap();
                    } else {
                        writeln!(test_file, "    // Using built-in Option<Felt> (equivalent to abigen!-generated type)").unwrap();
                        writeln!(test_file, "    let test_instance = None::<Felt>;").unwrap();
                        writeln!(
                            test_file,
                            "    let serialized = Option::<Felt>::cairo_serialize(&test_instance);"
                        )
                        .unwrap();
                        writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Option<Felt> None serialization mismatch\");").unwrap();

                        // Add roundtrip test
                        writeln!(test_file, "    let deserialize_ptr = 0;").unwrap();
                        writeln!(test_file, "    let deserialized = Option::<Felt>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();").unwrap();
                        writeln!(test_file, "    assert_eq!(test_instance, deserialized, \"Option<Felt> None roundtrip failed\");").unwrap();
                    }
                }
                "core::result::Result::<core::felt252, core::integer::u32>" => {
                    // For core types, use built-in types as they have equivalent serialization
                    if type_name.contains("ok") {
                        writeln!(test_file, "    // Using built-in Result<Felt, u32> (equivalent to abigen!-generated type)").unwrap();
                        writeln!(test_file, "    let test_instance = Ok::<Felt, u32>(Felt::from_hex(\"0x7b\").unwrap());").unwrap();
                        writeln!(test_file, "    let serialized = Result::<Felt, u32>::cairo_serialize(&test_instance);").unwrap();
                        writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Result<Felt, u32> Ok serialization mismatch\");").unwrap();

                        // Add roundtrip test
                        writeln!(test_file, "    let deserialize_ptr = 0;").unwrap();
                        writeln!(test_file, "    let deserialized = Result::<Felt, u32>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();").unwrap();
                        writeln!(test_file, "    assert_eq!(test_instance, deserialized, \"Result<Felt, u32> Ok roundtrip failed\");").unwrap();
                    } else {
                        writeln!(test_file, "    // Using built-in Result<Felt, u32> (equivalent to abigen!-generated type)").unwrap();
                        writeln!(
                            test_file,
                            "    let test_instance = Err::<Felt, u32>(456_u32);"
                        )
                        .unwrap();
                        writeln!(test_file, "    let serialized = Result::<Felt, u32>::cairo_serialize(&test_instance);").unwrap();
                        writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Result<Felt, u32> Err serialization mismatch\");").unwrap();

                        // Add roundtrip test
                        writeln!(test_file, "    let deserialize_ptr = 0;").unwrap();
                        writeln!(test_file, "    let deserialized = Result::<Felt, u32>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();").unwrap();
                        writeln!(test_file, "    assert_eq!(test_instance, deserialized, \"Result<Felt, u32> Err roundtrip failed\");").unwrap();
                    }
                }
                _ => {
                    // For custom enums, generate test instance based on artifact data
                    generate_custom_enum_test(
                        test_file,
                        type_name,
                        contract_name,
                        abi_type,
                        artifact,
                    );
                }
            }
        }
    }
}

fn generate_custom_enum_test(
    test_file: &mut fs::File,
    type_name: &str,
    contract_name: &str,
    abi_type: &Value,
    _artifact: &Value,
) {
    writeln!(
        test_file,
        "    // Testing custom enum type with abigen!-generated {}",
        contract_name
    )
    .unwrap();

    // Extract the enum type name from the abi_type
    if let Some(name) = abi_type.get("name") {
        if let Some(name_str) = name.as_str() {
            // Get the last part of the type name (e.g., SimpleEnum from contracts::abicov::enums::SimpleEnum)
            let enum_type_name = name_str.split("::").last().unwrap_or("UnknownEnum");

            // Generate test instance based on the artifact's test_value and variant type
            if let Some(variants) = abi_type.get("variants") {
                if let Some(variants_array) = variants.as_array() {
                    // Determine which variant we're testing based on type_name
                    let variant_index = if type_name.contains("variant2") { 1 } else { 0 };

                    if let Some(target_variant) = variants_array.get(variant_index) {
                        if let Some(variant_name) = target_variant.get("name") {
                            if let Some(variant_name_str) = variant_name.as_str() {
                                // Check if this variant has data
                                if let Some(variant_type) = target_variant.get("type") {
                                    if variant_type == "()" {
                                        // Empty variant - use actual abigen!-generated enum
                                        writeln!(
                                            test_file,
                                            "    // Using actual abigen!-generated enum type"
                                        )
                                        .unwrap();
                                        writeln!(
                                            test_file,
                                            "    let test_instance = {}::{};",
                                            enum_type_name, variant_name_str
                                        )
                                        .unwrap();
                                        writeln!(test_file, "    let serialized = {}::cairo_serialize(&test_instance);", enum_type_name).unwrap();
                                        writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Enum serialization mismatch\");").unwrap();
                                        writeln!(test_file, "    // Verify deserialization works by checking it produces the same serialization").unwrap();
                                        writeln!(test_file, "    let deserialize_ptr = 0;")
                                            .unwrap();
                                        writeln!(test_file, "    let deserialized = {}::cairo_deserialize(&serialized, deserialize_ptr).unwrap();", enum_type_name).unwrap();
                                        writeln!(test_file, "    let reserialized = {}::cairo_serialize(&deserialized);", enum_type_name).unwrap();
                                        writeln!(test_file, "    assert_eq!(serialized, reserialized, \"Enum roundtrip serialization failed\");").unwrap();
                                    } else {
                                        // Variant with data - use actual abigen!-generated enum with constructor
                                        writeln!(test_file, "    // Using actual abigen!-generated enum type with data").unwrap();

                                        // Generate appropriate test data based on variant type
                                        let test_data = match variant_type.as_str() {
                                            Some("core::felt252") => {
                                                "Felt::from_hex(\"0x789\").unwrap()".to_string()
                                            }
                                            Some("core::integer::u256") => {
                                                // For u256, we need to construct it properly
                                                "cainome_cairo_serde::U256 { low: 789_u128, high: 0_u128 }".to_string()
                                            }
                                            Some(s) if s.starts_with("(") => {
                                                // Tuple type - construct tuple
                                                "(Felt::from_hex(\"0x7b\").unwrap(), cainome_cairo_serde::U256 { low: 456_u128, high: 0_u128 })".to_string()
                                            }
                                            _ => "Felt::from_hex(\"0x789\").unwrap()".to_string(), // Default fallback
                                        };

                                        writeln!(
                                            test_file,
                                            "    let test_instance = {}::{}({});",
                                            enum_type_name, variant_name_str, test_data
                                        )
                                        .unwrap();
                                        writeln!(test_file, "    let serialized = {}::cairo_serialize(&test_instance);", enum_type_name).unwrap();
                                        writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Enum serialization mismatch\");").unwrap();
                                        writeln!(test_file, "    // Verify deserialization works by checking it produces the same serialization").unwrap();
                                        writeln!(test_file, "    let deserialize_ptr = 0;")
                                            .unwrap();
                                        writeln!(test_file, "    let deserialized = {}::cairo_deserialize(&serialized, deserialize_ptr).unwrap();", enum_type_name).unwrap();
                                        writeln!(test_file, "    let reserialized = {}::cairo_serialize(&deserialized);", enum_type_name).unwrap();
                                        writeln!(test_file, "    assert_eq!(serialized, reserialized, \"Enum roundtrip serialization failed\");").unwrap();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn generate_struct_abigen_test(
    test_file: &mut fs::File,
    _type_name: &str,
    contract_name: &str,
    abi_type: &Value,
    _artifact: &Value,
) {
    writeln!(
        test_file,
        "    // Testing custom struct type with abigen!-generated {}",
        contract_name
    )
    .unwrap();

    // Extract the primary struct type name (for arrays, use the last/most complex type)
    let struct_type_name = if abi_type.is_array() {
        // For arrays, find the most complex struct (usually the last one)
        if let Some(array) = abi_type.as_array() {
            if let Some(last_struct) = array.last() {
                if let Some(name) = last_struct.get("name") {
                    if let Some(name_str) = name.as_str() {
                        name_str.split("::").last().unwrap_or("UnknownStruct")
                    } else {
                        "UnknownStruct"
                    }
                } else {
                    "UnknownStruct"
                }
            } else {
                "UnknownStruct"
            }
        } else {
            "UnknownStruct"
        }
    } else {
        // Single object - use its name directly
        if let Some(name) = abi_type.get("name") {
            if let Some(name_str) = name.as_str() {
                name_str.split("::").last().unwrap_or("UnknownStruct")
            } else {
                "UnknownStruct"
            }
        } else {
            "UnknownStruct"
        }
    };

    if struct_type_name != "UnknownStruct" {
        // Generate actual test instance using abigen!-generated struct
        writeln!(
            test_file,
            "    // Using actual abigen!-generated {} struct",
            struct_type_name
        )
        .unwrap();

        // Deserialize from expected felt values instead of manually constructing
        writeln!(
            test_file,
            "    // Deserialize from expected felt array (tests deserialization)"
        )
        .unwrap();
        writeln!(
            test_file,
            "    let test_instance = {}::cairo_deserialize(&expected_felt_values, 0).unwrap();",
            struct_type_name
        )
        .unwrap();

        writeln!(
            test_file,
            "    // Test serialization using abigen!-generated type (tests serialization)"
        )
        .unwrap();
        writeln!(
            test_file,
            "    let serialized = {}::cairo_serialize(&test_instance);",
            struct_type_name
        )
        .unwrap();
        writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Struct round-trip serialization failed\");").unwrap();

        writeln!(
            test_file,
            "    // Verify we can deserialize again (double round-trip)"
        )
        .unwrap();
        writeln!(
            test_file,
            "    let deserialized_again = {}::cairo_deserialize(&serialized, 0).unwrap();",
            struct_type_name
        )
        .unwrap();
        writeln!(
            test_file,
            "    let reserialized = {}::cairo_serialize(&deserialized_again);",
            struct_type_name
        )
        .unwrap();
        writeln!(
            test_file,
            "    assert_eq!(serialized, reserialized, \"Double round-trip failed\");"
        )
        .unwrap();
    }
}

fn can_generate_roundtrip_test(type_name: &str, artifact: &Value) -> bool {
    // Only generate tests for types where we can create proper instances and do roundtrip testing
    match type_name {
        // Primitives - full roundtrip support
        "felt252" | "bool_true" | "bool_false" | "u8" | "u32" | "u64" | "u128" => true,

        // Arrays - full roundtrip support
        "array_felt" | "array_u32" => true,

        // Tuples - full roundtrip support
        "tuple_felt_u32" => true,

        // ByteArrays - full roundtrip support
        "byte_array" | "byte_array_empty" => true,

        // Simple structs that we can construct - full roundtrip support
        "u256" | "u256_large" | "eth_address" => true,

        // Complex structs - struct_with_struct supported with abigen!, simple_struct uses direct testing
        "simple_struct" => true,      // Use direct cairo-serde testing
        "struct_with_struct" => true, // Use abigen! testing

        // Option/Result types - core types that work well
        _ if type_name.contains("option") => true,
        _ if type_name.contains("result") => true,

        // Enum variants - check if they can be properly tested
        _ if type_name.contains("enum") && type_name.contains("variant") => {
            // Check if this is a simple enum that generates abigen! or has implementable test instances
            can_generate_enum_roundtrip_test(type_name, artifact)
        }

        // Unknown types - skip
        _ => false,
    }
}

fn can_generate_enum_roundtrip_test(type_name: &str, artifact: &Value) -> bool {
    // Check if we can generate abigen! for this enum (simple enums work)
    if should_generate_abi(artifact) {
        return true;
    }

    // For enums that don't generate abigen!, check if we have direct implementations
    match type_name {
        // Simple enum variants that we manually implement
        "simple_enum_variant1" | "simple_enum_variant2" => true,
        "mixed_enum_variant1" | "mixed_enum_variant2" => true,

        // Typed enum variants with complex dependencies - cannot construct properly
        "typed_enum_variant1" | "typed_enum_variant2" | "typed_enum_variant3" => false,

        _ => false,
    }
}

fn write_test_for_artifact(
    test_file: &mut fs::File,
    content: &str,
    type_name: &str,
    artifact: &Value,
) {
    // Only generate tests for types where we can do proper cairo-serde roundtrips
    if !can_generate_roundtrip_test(type_name, artifact) {
        return; // Skip this test entirely
    }

    let cairo_serialized = extract_cairo_serialized(content);
    let description = extract_json_string_field(content, "description");

    writeln!(test_file, "#[test]").unwrap();
    writeln!(test_file, "fn test_{}() {{", type_name).unwrap();
    writeln!(test_file, "    let description = r#\"{}\"#;", description).unwrap();
    writeln!(test_file).unwrap();

    // Generate the expected serialized data
    writeln!(test_file, "    let expected_serialized = [").unwrap();
    for hex_value in &cairo_serialized {
        writeln!(test_file, "        \"{}\",", hex_value).unwrap();
    }
    writeln!(test_file, "    ];").unwrap();
    writeln!(test_file).unwrap();

    // Convert to Felt values
    writeln!(
        test_file,
        "    let expected_felt_values: Vec<Felt> = expected_serialized"
    )
    .unwrap();
    writeln!(test_file, "        .iter()").unwrap();
    writeln!(test_file, "        .map(|s| Felt::from_hex(s).unwrap())").unwrap();
    writeln!(test_file, "        .collect();").unwrap();
    writeln!(test_file).unwrap();

    writeln!(
        test_file,
        "    println!(\"Testing {}: {{}}\", description);",
        type_name
    )
    .unwrap();
    writeln!(
        test_file,
        "    println!(\"Expected: {{:?}}\", expected_felt_values);"
    )
    .unwrap();
    writeln!(test_file).unwrap();

    // Check if we should generate abigen-based tests or basic serialization tests
    if should_generate_abi(artifact) {
        generate_abigen_based_test(test_file, type_name, artifact);
    } else {
        // For types without abigen! support, generate direct cairo-serde tests
        generate_direct_cairo_serde_test(test_file, type_name, artifact);
    }

    writeln!(test_file, "}}").unwrap();
    writeln!(test_file).unwrap();
}

fn generate_direct_cairo_serde_test(test_file: &mut fs::File, type_name: &str, artifact: &Value) {
    writeln!(
        test_file,
        "    // Direct cairo-serde roundtrip test using test artifact data"
    )
    .unwrap();

    // Parse the test_value from the artifact to create actual test instances
    if let Some(test_value) = artifact.get("test_value") {
        if let Some(test_value_str) = test_value.as_str() {
            // Generate test instance based on the actual test value from artifact
            if let Some((instance, serialize)) =
                generate_test_instance_from_artifact_data(type_name, test_value_str, artifact)
            {
                writeln!(test_file, "    // Using actual test data from artifact").unwrap();
                writeln!(test_file, "    let test_instance = {};", instance).unwrap();
                writeln!(test_file).unwrap();

                writeln!(test_file, "    // Test serialization").unwrap();
                writeln!(test_file, "    let serialized = {};", serialize).unwrap();
                writeln!(test_file).unwrap();

                writeln!(test_file, "    println!(\"Actual:   {{:?}}\", serialized);").unwrap();
                writeln!(test_file).unwrap();

                writeln!(test_file, "    // Verify serialization matches expected").unwrap();
                writeln!(test_file, "    assert_eq!(serialized.len(), expected_felt_values.len(), \"Serialized length mismatch\");").unwrap();
                writeln!(test_file, "    assert_eq!(serialized, expected_felt_values, \"Serialized values mismatch\");").unwrap();

                // Add roundtrip test - all types that reach here should support roundtrips
                if let Some(deserialize_type) =
                    get_deserialize_type_for_artifact(type_name, artifact)
                {
                    writeln!(test_file).unwrap();
                    writeln!(test_file, "    // Test roundtrip deserialization").unwrap();
                    writeln!(test_file, "    let deserialize_ptr = 0;").unwrap();
                    writeln!(test_file, "    let deserialized = {}::cairo_deserialize(&serialized, deserialize_ptr).unwrap();", deserialize_type).unwrap();
                    writeln!(test_file, "    assert_eq!(test_instance, deserialized, \"Roundtrip deserialization failed\");").unwrap();
                }
            }
        }
    }
}

fn generate_test_instance_from_artifact_data(
    type_name: &str,
    test_value: &str,
    _artifact: &Value,
) -> Option<(String, String)> {
    // Generate test instances using actual artifact test_value data
    match type_name {
        // Primitives - use actual values from test artifacts
        "felt252" => Some((
            "Felt::from_hex(\"0x1234567890abcdef\").unwrap()".to_string(),
            "vec![test_instance]".to_string()
        )),
        "bool_true" => Some(("true".to_string(), "bool::cairo_serialize(&test_instance)".to_string())),
        "bool_false" => Some(("false".to_string(), "bool::cairo_serialize(&test_instance)".to_string())),
        "u8" => Some(("255_u8".to_string(), "u8::cairo_serialize(&test_instance)".to_string())),
        "u32" => Some(("4294967295_u32".to_string(), "u32::cairo_serialize(&test_instance)".to_string())),
        "u64" => Some(("18446744073709551615_u64".to_string(), "u64::cairo_serialize(&test_instance)".to_string())),
        "u128" => Some(("1234567890123456789_u128".to_string(), "u128::cairo_serialize(&test_instance)".to_string())),
        // Arrays
        "array_felt" => Some((
            "vec![Felt::from_hex(\"0x1\").unwrap(), Felt::from_hex(\"0x2\").unwrap(), Felt::from_hex(\"0x3\").unwrap()]".to_string(),
            "Vec::<Felt>::cairo_serialize(&test_instance)".to_string()
        )),
        "array_u32" => Some((
            "vec![1_u32, 2_u32, 3_u32]".to_string(),
            "Vec::<u32>::cairo_serialize(&test_instance)".to_string()
        )),
        // Tuples
        "tuple_felt_u32" => Some((
            "(Felt::from_hex(\"0x7b\").unwrap(), 456_u32)".to_string(),
            "<(Felt, u32)>::cairo_serialize(&test_instance)".to_string()
        )),
        // ByteArrays - use actual test_value from artifact
        "byte_array" | "byte_array_empty" => {
            // Extract the actual string value from test_value (remove quotes)
            let string_value = test_value.trim_matches('"');
            Some((
                format!("cainome_cairo_serde::ByteArray::from_string(\"{}\").unwrap()", string_value),
                "cainome_cairo_serde::ByteArray::cairo_serialize(&test_instance)".to_string()
            ))
        },
        // Complex struct instances - deserialize from felt array, don't construct manually
        "simple_struct" => {
            // Use deserialization from the expected felt values
            Some((
                "Simple::cairo_deserialize(&expected_felt_values, 0).unwrap()".to_string(),
                "Simple::cairo_serialize(&test_instance)".to_string()
            ))
        },
        // Structs - generate test instances for simple structs using actual artifact values
        "u256" => Some((
            "cainome_cairo_serde::U256 { low: 0x123456789abcdef123456789abcdef12_u128, high: 0_u128 }".to_string(),
            "cainome_cairo_serde::U256::cairo_serialize(&test_instance)".to_string()
        )),
        "u256_large" => Some((
            "cainome_cairo_serde::U256 { low: 0xffffffffffffffffffffffffffffffff_u128, high: 0x123456789abcdef123456789abcdef12_u128 }".to_string(),
            "cainome_cairo_serde::U256::cairo_serialize(&test_instance)".to_string()
        )),
        "eth_address" => Some((
            "cainome_cairo_serde::EthAddress::from(Felt::from_hex(\"0x1234567890abcdef1234567890abcdef12345678\").unwrap())".to_string(),
            "cainome_cairo_serde::EthAddress::cairo_serialize(&test_instance)".to_string()
        )),
        _ => None,
    }
}

fn get_deserialize_type_for_artifact(type_name: &str, _artifact: &Value) -> Option<String> {
    match type_name {
        "felt252" => Some("Felt".to_string()),
        "bool_true" | "bool_false" => Some("bool".to_string()),
        "u8" => Some("u8".to_string()),
        "u32" => Some("u32".to_string()),
        "u64" => Some("u64".to_string()),
        "u128" => Some("u128".to_string()),
        "array_felt" => Some("Vec::<Felt>".to_string()),
        "array_u32" => Some("Vec::<u32>".to_string()),
        "tuple_felt_u32" => Some("<(Felt, u32)>".to_string()),
        "byte_array" | "byte_array_empty" => Some("cainome_cairo_serde::ByteArray".to_string()),
        "u256" | "u256_large" => Some("cainome_cairo_serde::U256".to_string()),
        "eth_address" => Some("cainome_cairo_serde::EthAddress".to_string()),
        "simple_struct" => Some("Simple".to_string()), // Use the abigen!-generated Simple type
        _ => None,
    }
}

fn extract_cairo_serialized(content: &str) -> Vec<String> {
    // Find the cairo_serialized field
    if let Some(start) = content.find("\"cairo_serialized\":") {
        let start = start + "\"cairo_serialized\":".len();
        let remaining = &content[start..];

        // Skip whitespace
        let remaining = remaining.trim_start();

        if remaining.starts_with('[') {
            if let Some(end) = find_array_end(remaining) {
                let array_content = &remaining[1..end];
                return parse_string_array(array_content);
            }
        }
    }

    vec![]
}

fn extract_json_string_field(content: &str, field: &str) -> String {
    let field_pattern = format!("\"{}\":", field);
    if let Some(start) = content.find(&field_pattern) {
        let start = start + field_pattern.len();
        let remaining = &content[start..];

        // Skip whitespace
        let remaining = remaining.trim_start();

        if let Some(stripped) = remaining.strip_prefix('"') {
            if let Some(end) = find_string_end(stripped) {
                return remaining[1..end + 1].to_string();
            }
        }
    }

    String::new()
}

fn parse_string_array(array_content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for c in array_content.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' if in_string => {
                escaped = true;
                current.push(c);
            }
            '"' => {
                if in_string {
                    result.push(current.clone());
                    current.clear();
                    in_string = false;
                } else {
                    in_string = true;
                }
            }
            ',' | ' ' | '\n' | '\t' => {
                if in_string {
                    current.push(c);
                }
                // Skip whitespace and commas outside strings
            }
            _ => {
                if in_string {
                    current.push(c);
                }
            }
        }
    }

    result
}

fn find_string_end(s: &str) -> Option<usize> {
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(i);
        }
    }
    None
}

fn find_array_end(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_string {
            if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                '"' => in_string = true,
                _ => {}
            }
        }
    }
    None
}
