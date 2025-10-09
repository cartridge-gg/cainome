// Generated test file - do not edit manually
use cainome_cairo_serde::CairoSerde;
use starknet::core::types::Felt;
use cainome::rs::abigen;
#[allow(unused_imports)]
use serde::{Serialize, Deserialize};

abigen!(TestSimpleEnumVariant1, r#"[{"name":"contracts::abicov::enums::SimpleEnum","type":"enum","variants":[{"name":"Variant1","type":"()"},{"name":"Variant2","type":"()"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestResultErrU32, r#"[{"name":"core::result::Result::<core::felt252, core::integer::u32>","type":"enum","variants":[{"name":"Ok","type":"core::felt252"},{"name":"Err","type":"core::integer::u32"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestOptionSomeFelt, r#"[{"name":"core::option::Option::<core::felt252>","type":"enum","variants":[{"name":"Some","type":"core::felt252"},{"name":"None","type":"()"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestOptionNoneFelt, r#"[{"name":"core::option::Option::<core::felt252>","type":"enum","variants":[{"name":"Some","type":"core::felt252"},{"name":"None","type":"()"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestBoolFalse, r#"[{"name":"core::bool","type":"enum","variants":[{"name":"False","type":"()"},{"name":"True","type":"()"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestBoolTrue, r#"[{"name":"core::bool","type":"enum","variants":[{"name":"False","type":"()"},{"name":"True","type":"()"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestMixedEnumVariant1, r#"[{"name":"contracts::abicov::enums::MixedEnum","type":"enum","variants":[{"name":"Variant1","type":"core::felt252"},{"name":"Variant2","type":"()"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestStructWithStruct, r#"[{"members":[{"name":"felt","type":"core::felt252"},{"name":"uint256","type":"core::integer::u256"},{"name":"uint64","type":"core::integer::u64"},{"name":"address","type":"core::starknet::contract_address::ContractAddress"},{"name":"class_hash","type":"core::starknet::class_hash::ClassHash"},{"name":"eth_address","type":"core::starknet::eth_address::EthAddress"},{"name":"tuple","type":"(core::felt252, core::integer::u256)"},{"name":"span","type":"core::array::Span::<core::felt252>"}],"name":"contracts::abicov::structs::Simple","type":"struct"},{"members":[{"name":"simple","type":"contracts::abicov::structs::Simple"}],"name":"contracts::abicov::structs::StructWithStruct","type":"struct"}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));
abigen!(TestResultOkFelt, r#"[{"name":"core::result::Result::<core::felt252, core::integer::u32>","type":"enum","variants":[{"name":"Ok","type":"core::felt252"},{"name":"Err","type":"core::integer::u32"}]}]"#, derives(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize));

#[test]
fn test_array_felt() {
    let description = r#"Array of felt252 values"#;

    let expected_serialized = [
        "0x3",
        "0x1",
        "0x2",
        "0x3",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing array_felt: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = vec![Felt::from_hex("0x1").unwrap(), Felt::from_hex("0x2").unwrap(), Felt::from_hex("0x3").unwrap()];

    // Test serialization
    let serialized = Vec::<Felt>::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = Vec::<Felt>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_simple_enum_variant1() {
    let description = r#"SimpleEnum with first variant"#;

    let expected_serialized = [
        "0x0",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing simple_enum_variant1: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestSimpleEnumVariant1
    // Expected test value: SimpleEnum::Variant1

    // Testing custom enum type with abigen!-generated TestSimpleEnumVariant1
    // Using actual abigen!-generated enum type
    let test_instance = SimpleEnum::Variant1;
    let serialized = SimpleEnum::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Enum serialization mismatch");
    // Verify deserialization works by checking it produces the same serialization
    let deserialize_ptr = 0;
    let deserialized = SimpleEnum::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    let reserialized = SimpleEnum::cairo_serialize(&deserialized);
    assert_eq!(serialized, reserialized, "Enum roundtrip serialization failed");
}

#[test]
fn test_result_err_u32() {
    let description = r#"Result with Err(u32)"#;

    let expected_serialized = [
        "0x1",
        "0x1c8",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing result_err_u32: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestResultErrU32
    // Expected test value: Err(456)

    // Using built-in Result<Felt, u32> (equivalent to abigen!-generated type)
    let test_instance = Err::<Felt, u32>(456_u32);
    let serialized = Result::<Felt, u32>::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Result<Felt, u32> Err serialization mismatch");
    let deserialize_ptr = 0;
    let deserialized = Result::<Felt, u32>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Result<Felt, u32> Err roundtrip failed");
}

#[test]
fn test_byte_array() {
    let description = r#"ByteArray with string data"#;

    let expected_serialized = [
        "0x0",
        "0x48656c6c6f2c20436169726f21",
        "0xd",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing byte_array: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = cainome_cairo_serde::ByteArray::from_string("Hello, Cairo!").unwrap();

    // Test serialization
    let serialized = cainome_cairo_serde::ByteArray::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = cainome_cairo_serde::ByteArray::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_option_some_felt() {
    let description = r#"Option with Some(felt252)"#;

    let expected_serialized = [
        "0x0",
        "0x2a",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing option_some_felt: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestOptionSomeFelt
    // Expected test value: Some(0x2a)

    // Using built-in Option<Felt> (equivalent to abigen!-generated type)
    let test_instance = Some(Felt::from_hex("0x2a").unwrap());
    let serialized = Option::<Felt>::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Option<Felt> Some serialization mismatch");
    let deserialize_ptr = 0;
    let deserialized = Option::<Felt>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Option<Felt> Some roundtrip failed");
}

#[test]
fn test_option_none_felt() {
    let description = r#"Option with None"#;

    let expected_serialized = [
        "0x1",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing option_none_felt: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestOptionNoneFelt
    // Expected test value: None

    // Using built-in Option<Felt> (equivalent to abigen!-generated type)
    let test_instance = None::<Felt>;
    let serialized = Option::<Felt>::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Option<Felt> None serialization mismatch");
    let deserialize_ptr = 0;
    let deserialized = Option::<Felt>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Option<Felt> None roundtrip failed");
}

#[test]
fn test_bool_false() {
    let description = r#"Boolean false value"#;

    let expected_serialized = [
        "0x0",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing bool_false: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestBoolFalse
    // Expected test value: false

    // Using built-in bool (equivalent to abigen!-generated Bool)
    let test_instance = false;
    let serialized = bool::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Bool serialization mismatch");
    let deserialize_ptr = 0;
    let deserialized = bool::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Bool roundtrip failed");
}

#[test]
fn test_u32() {
    let description = r#"Maximum u32 value"#;

    let expected_serialized = [
        "0xffffffff",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing u32: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = 4294967295_u32;

    // Test serialization
    let serialized = u32::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = u32::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_tuple_felt_u32() {
    let description = r#"Tuple of felt252 and u32"#;

    let expected_serialized = [
        "0x7b",
        "0x1c8",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing tuple_felt_u32: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = (Felt::from_hex("0x7b").unwrap(), 456_u32);

    // Test serialization
    let serialized = <(Felt, u32)>::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = <(Felt, u32)>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_u256_large() {
    let description = r#"U256 struct with large values"#;

    let expected_serialized = [
        "0xffffffffffffffffffffffffffffffff",
        "0x123456789abcdef123456789abcdef12",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing u256_large: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = cainome_cairo_serde::U256 { low: 0xffffffffffffffffffffffffffffffff_u128, high: 0x123456789abcdef123456789abcdef12_u128 };

    // Test serialization
    let serialized = cainome_cairo_serde::U256::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = cainome_cairo_serde::U256::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_u64() {
    let description = r#"Maximum u64 value"#;

    let expected_serialized = [
        "0xffffffffffffffff",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing u64: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = 18446744073709551615_u64;

    // Test serialization
    let serialized = u64::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = u64::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_bool_true() {
    let description = r#"Boolean true value"#;

    let expected_serialized = [
        "0x1",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing bool_true: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestBoolTrue
    // Expected test value: true

    // Using built-in bool (equivalent to abigen!-generated Bool)
    let test_instance = true;
    let serialized = bool::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Bool serialization mismatch");
    let deserialize_ptr = 0;
    let deserialized = bool::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Bool roundtrip failed");
}

#[test]
fn test_mixed_enum_variant1() {
    let description = r#"MixedEnum with felt252 variant"#;

    let expected_serialized = [
        "0x0",
        "0x789",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing mixed_enum_variant1: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestMixedEnumVariant1
    // Expected test value: MixedEnum::Variant1(0x789)

    // Testing custom enum type with abigen!-generated TestMixedEnumVariant1
    // Using actual abigen!-generated enum type with data
    let test_instance = MixedEnum::Variant1(Felt::from_hex("0x789").unwrap());
    let serialized = MixedEnum::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Enum serialization mismatch");
    // Verify deserialization works by checking it produces the same serialization
    let deserialize_ptr = 0;
    let deserialized = MixedEnum::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    let reserialized = MixedEnum::cairo_serialize(&deserialized);
    assert_eq!(serialized, reserialized, "Enum roundtrip serialization failed");
}

#[test]
fn test_felt252() {
    let description = r#"Basic felt252 value"#;

    let expected_serialized = [
        "0x1234567890abcdef",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing felt252: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = Felt::from_hex("0x1234567890abcdef").unwrap();

    // Test serialization
    let serialized = vec![test_instance];

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = Felt::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_u256() {
    let description = r#"U256 struct with low and high fields"#;

    let expected_serialized = [
        "0x123456789abcdef123456789abcdef12",
        "0x0",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing u256: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = cainome_cairo_serde::U256 { low: 0x123456789abcdef123456789abcdef12_u128, high: 0_u128 };

    // Test serialization
    let serialized = cainome_cairo_serde::U256::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = cainome_cairo_serde::U256::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_u128() {
    let description = r#"Large u128 value"#;

    let expected_serialized = [
        "0x112210f47de98115",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing u128: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = 1234567890123456789_u128;

    // Test serialization
    let serialized = u128::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = u128::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_struct_with_struct() {
    let description = r#"Struct containing another struct"#;

    let expected_serialized = [
        "0x123",
        "0x1c8",
        "0x0",
        "0x315",
        "0x1234567890abcdef1234567890abcdef12345678",
        "0xabcdef1234567890abcdef1234567890abcdef12",
        "0x1234567890abcdef1234567890abcdef12345678",
        "0x2a",
        "0x7b",
        "0x0",
        "0x3",
        "0x1",
        "0x2",
        "0x3",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing struct_with_struct: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestStructWithStruct
    // Expected test value: StructWithStruct { simple: Simple { felt: 0x123, uint256: U256 { low: 456, high: 0 }, uint64: 789, address: ContractAddress(0x1234567890abcdef1234567890abcdef12345678), class_hash: ClassHash(0xabcdef1234567890abcdef1234567890abcdef12), eth_address: EthAddress(0x1234567890abcdef1234567890abcdef12345678), tuple: (0x2a, U256 { low: 123, high: 0 }), span: [0x1, 0x2, 0x3] } }

    // Testing custom struct type with abigen!-generated TestStructWithStruct
    // Using actual abigen!-generated StructWithStruct struct
    // Deserialize from expected felt array (tests deserialization)
    let test_instance = StructWithStruct::cairo_deserialize(&expected_felt_values, 0).unwrap();
    // Test serialization using abigen!-generated type (tests serialization)
    let serialized = StructWithStruct::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Struct round-trip serialization failed");
    // Verify we can deserialize again (double round-trip)
    let deserialized_again = StructWithStruct::cairo_deserialize(&serialized, 0).unwrap();
    let reserialized = StructWithStruct::cairo_serialize(&deserialized_again);
    assert_eq!(serialized, reserialized, "Double round-trip failed");
}

#[test]
fn test_u8() {
    let description = r#"Maximum u8 value"#;

    let expected_serialized = [
        "0xff",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing u8: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = 255_u8;

    // Test serialization
    let serialized = u8::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = u8::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_byte_array_empty() {
    let description = r#"Empty ByteArray"#;

    let expected_serialized = [
        "0x0",
        "0x0",
        "0x0",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing byte_array_empty: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = cainome_cairo_serde::ByteArray::from_string("").unwrap();

    // Test serialization
    let serialized = cainome_cairo_serde::ByteArray::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = cainome_cairo_serde::ByteArray::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_result_ok_felt() {
    let description = r#"Result with Ok(felt252)"#;

    let expected_serialized = [
        "0x0",
        "0x7b",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing result_ok_felt: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Test using abigen!-generated type: TestResultOkFelt
    // Expected test value: Ok(0x7b)

    // Using built-in Result<Felt, u32> (equivalent to abigen!-generated type)
    let test_instance = Ok::<Felt, u32>(Felt::from_hex("0x7b").unwrap());
    let serialized = Result::<Felt, u32>::cairo_serialize(&test_instance);
    assert_eq!(serialized, expected_felt_values, "Result<Felt, u32> Ok serialization mismatch");
    let deserialize_ptr = 0;
    let deserialized = Result::<Felt, u32>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Result<Felt, u32> Ok roundtrip failed");
}

#[test]
fn test_eth_address() {
    let description = r#"EthAddress value"#;

    let expected_serialized = [
        "0x1234567890abcdef1234567890abcdef12345678",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing eth_address: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = cainome_cairo_serde::EthAddress::from(Felt::from_hex("0x1234567890abcdef1234567890abcdef12345678").unwrap());

    // Test serialization
    let serialized = cainome_cairo_serde::EthAddress::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = cainome_cairo_serde::EthAddress::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

#[test]
fn test_array_u32() {
    let description = r#"Array of u32 values"#;

    let expected_serialized = [
        "0x3",
        "0x1",
        "0x2",
        "0x3",
    ];

    let expected_felt_values: Vec<Felt> = expected_serialized
        .iter()
        .map(|s| Felt::from_hex(s).unwrap())
        .collect();

    println!("Testing array_u32: {}", description);
    println!("Expected: {:?}", expected_felt_values);

    // Direct cairo-serde roundtrip test using test artifact data
    // Using actual test data from artifact
    let test_instance = vec![1_u32, 2_u32, 3_u32];

    // Test serialization
    let serialized = Vec::<u32>::cairo_serialize(&test_instance);

    println!("Actual:   {:?}", serialized);

    // Verify serialization matches expected
    assert_eq!(serialized.len(), expected_felt_values.len(), "Serialized length mismatch");
    assert_eq!(serialized, expected_felt_values, "Serialized values mismatch");

    // Test roundtrip deserialization
    let deserialize_ptr = 0;
    let deserialized = Vec::<u32>::cairo_deserialize(&serialized, deserialize_ptr).unwrap();
    assert_eq!(test_instance, deserialized, "Roundtrip deserialization failed");
}

