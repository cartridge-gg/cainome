use crate::abi::registry::{get_all_generic_inner_types, get_generic_inner_types};

#[test]
fn test_get_all_generic_inner_types_for_span_with_generic_struct() {
    let type_path = "(core::array::Span::<contracts::abicov::structs::GenericOne::<core::integer::u64>>, core::array::Span::<core::felt252>)";
    let data = get_all_generic_inner_types(type_path).unwrap();
    assert_eq!(data.len(), 3);
}

#[test]
fn test_get_all_generic_inner_types_for_span() {
    let type_path = "core::array::Span::<core::integer::u64>";
    let data = get_all_generic_inner_types(type_path).unwrap();
    assert_eq!(data, ["core::integer::u64"].to_vec());
}

#[test]
fn test_get_all_generic_inner_types_for_span_in_a_span() {
    let type_path = "core::array::Span::<core::array::Span::<core::integer::u64>>";
    let data = get_all_generic_inner_types(type_path).unwrap();
    assert_eq!(data, ["core::integer::u64"].to_vec());
}

#[test]
fn test_get_all_generic_inner_types_for_struct() {
    let type_path = "contracts::abicov::structs::GenericOne::<core::integer::u64>";
    let data = get_all_generic_inner_types(type_path).unwrap();
    assert_eq!(
        data,
        [
            "core::integer::u64",
            "contracts::abicov::structs::GenericOne"
        ]
        .to_vec()
    );
}

#[test]
fn test_get_all_generic_inner_types_for_tuple() {
    let type_path = "(core::integer::u64, core::integer::u64)";
    let data = get_all_generic_inner_types(type_path).unwrap();
    assert_eq!(data, ["core::integer::u64", "core::integer::u64"].to_vec());
}

#[test]
fn test_get_all_generic_inner_types_for_span_in_a_tuple() {
    let type_path =
        "(core::array::Span::<core::integer::u64>, core::array::Span::<core::integer::u64>)";
    let data = get_all_generic_inner_types(type_path).unwrap();
    assert_eq!(data, ["core::integer::u64", "core::integer::u64"].to_vec());
}

#[test]
fn test_get_generic_inner_types_for_span_in_a_tuple() {
    let type_path =
        "(core::array::Span::<core::integer::u64>, core::array::Span::<core::integer::u64>)";
    let data = get_generic_inner_types(type_path).unwrap();
    assert_eq!(data, ["core::integer::u64", "core::integer::u64"].to_vec());
}
