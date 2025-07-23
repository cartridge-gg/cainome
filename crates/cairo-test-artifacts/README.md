# Cainome Test Artifacts

This document describes the test artifacts system for Cainome, which provides ground truth Cairo serialization data for testing consistency across different language implementations.

## Overview

The test artifacts system generates JSON files containing the raw Cairo representation (felt arrays) for various types, which can be used by any language implementation to test round-trip serialization/deserialization.

## Architecture

The system consists of:

**Test Artifacts** - JSON files containing the serialized data that serve as ground truth for all language implementations

## Test Artifact Format

Each test artifact is a JSON file with the following structure:

```json
{
  "type_name": "string",           // Name of the type being tested
  "description": "string",         // Human-readable description
  "abi_type": {                    // ABI type definition for the Cairo type
    "type": "string",              // The type category (struct, enum, felt252, etc.)
    "name": "string"               // The fully qualified type name
  },
  "cairo_serialized": ["string"], // Array of felt values as hex strings
  "cairo_serialized_size": number  // Number of felts in the serialized representation
}
```

## Available Test Artifacts

All test artifacts are stored in the `test_artifacts/` directory:

### Basic Types

- **Primitive Types**: `felt252`, `bool_true`, `bool_false`, `u8`, `u16`, `u32`, `u64`, `u128`
- **Collections**: `array_felt`, `array_u32`
- **Tuples**: `tuple_felt_u32`
- **Optional Types**: `option_some_felt`, `option_none_felt`
- **Result Types**: `result_ok_felt`, `result_err_u32`
- **Byte Arrays**: `byte_array`, `byte_array_empty`
- **StarkNet Types**: `contract_address`, `class_hash`, `eth_address`

### Complex Types

- **Structs**: `simple_struct`, `struct_with_struct`
- **Enums**: `simple_enum_variant1`, `simple_enum_variant2`, `typed_enum_variant1`, `typed_enum_variant2`, `typed_enum_variant3`, `mixed_enum_variant1`, `mixed_enum_variant2`
- **Large Numbers**: `u256`, `u256_large`

## ABI Type Categories

The `abi_type` field contains type definitions extracted from compiled Cairo contracts with the following categories:

### Basic Types

- **Primitive**: `felt252`, `u8`, `u16`, `u32`, `u64`, `u128`
- **Struct**: Composite types with named fields (e.g., `u256`, `ContractAddress`)
- **Enum**: Union types with variants (e.g., `bool`, `Option`, `Result`)

### Complex Types

- **Generic**: Parameterized types (e.g., `Array<T>`)
- **Tuple**: Fixed-size composite types with positional fields
- **Contract Types**: User-defined structs and enums from compiled contracts

### ABI Source

The ABI type definitions are extracted from all compiled contract class files in:

- `contracts/target/dev/*.contract_class.json`

It includes:

- **Struct types**: Complete type definitions with member names and types
- **Enum types**: Complete type definitions with variant names and types
- **Built-in types**: Minimal fallback definitions for core types not in contracts

This ensures that the ABI types exactly match what the Cairo compiler produces and automatically includes new types as contracts are added.

## Usage

### For Language Implementers

1.  **Interface Generation**: Use the `abi_type` field to generate language-specific type definitions and interfaces
2.  **Use as Reference**: The `cairo_serialized` field contains the exact felt array representation that your language implementation should produce
3.  **Round-trip Testing**: Deserialize the felt array in your language and verify serialization produces the same result

### Example Test Pattern

```python
# Python example (pseudo-code)
def test_felt252_serialization():
    # Load test artifact
    with open('test_artifacts/felt252.json', 'r') as f:
        artifact = json.load(f)

    # Generate type interface from ABI
    abi_type = artifact['abi_type']
    interface = generate_interface(abi_type)  # Generate Python class/type

    # Test deserialization
    felts = [int(x, 16) for x in artifact['cairo_serialized']]
    result = deserialize_felt252(felts)

    # Test serialization
    serialized = serialize_felt252(result)
    assert serialized == felts
```

## Implementation Details

### Cairo Serialization Rules

Understanding the serialization format:

1.  **Primitive Types**: Most primitives serialize to a single felt
2.  **Booleans**: `false` = 0, `true` = 1
3.  **Arrays**: Length prefix followed by elements
4.  **Tuples**: Elements serialized in order
5.  **Structs**: Fields serialized in declaration order
6.  **Enums**: Variant index followed by variant data
7.  **Options**: `None` = 0, `Some(x)` = 0 followed by x
8.  **Results**: `Ok(x)` = 0 followed by x, `Err(e)` = 1 followed by e

### Example Serialization Breakdown

From `array_felt.json`:

```json
{
  "cairo_serialized": ["0x3", "0x1", "0x2", "0x3"],
  "cairo_serialized_size": 4
}
```

This shows:

- First felt (0x3) = array length
- Next 3 felts = array elements

From `simple_struct.json` (struct with 8 fields):

```json
{
  "cairo_serialized": [
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
    "0x3"
  ],
  "cairo_serialized_size": 14
}
```

This shows how complex structs are flattened into a sequence of felts.

## Validation

The test artifacts serve as the canonical reference for Cairo serialization. Any discrepancies between language implementations should be resolved by referring to these artifacts, which are generated directly from the Rust reference implementation.

## Future Enhancements

- **Parametric Types**: Support for generics and type parameters
- **Custom Derives**: Support for custom serialization logic
- **Performance Benchmarks**: Timing data for serialization operations
- **Compatibility Matrix**: Track which language implementations support which types
