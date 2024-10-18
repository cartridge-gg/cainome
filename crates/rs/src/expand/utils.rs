//! Utils function for expansion.
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, LitInt, LitStr, Type};

pub fn str_to_ident(str_in: &str) -> Ident {
    Ident::new(str_in, proc_macro2::Span::call_site())
}

pub fn str_to_type(str_in: &str) -> Type {
    syn::parse_str(str_in).unwrap_or_else(|_| panic!("Can't convert {} to syn::Type", str_in))
}

pub fn str_to_litstr(str_in: &str) -> LitStr {
    LitStr::new(str_in, proc_macro2::Span::call_site())
}

pub fn str_to_litint(str_in: &str) -> LitInt {
    LitInt::new(str_in, proc_macro2::Span::call_site())
}

pub fn snrs_types() -> Type {
    str_to_type("starknet::core::types")
}

pub fn snrs_accounts() -> Type {
    str_to_type("starknet::accounts")
}

pub fn snrs_providers() -> Type {
    str_to_type("starknet::providers")
}

pub fn cainome_cairo_serde() -> Type {
    str_to_type(&cainome_cairo_serde_path())
}

#[inline]
pub fn cainome_cairo_serde_path() -> String {
    //String::from("cainome_cairo_serde")
    String::from("cainome::cairo_serde")
}

/// Expands the implementation line with generic types.
pub fn impl_with_gen_args(entity_name: &Ident, gen_args: &Vec<Ident>) -> TokenStream2 {
    let gen_args_rust: Vec<Ident> = gen_args
        .iter()
        .map(|g| str_to_ident(format!("R{}", g).as_str()))
        .collect();

    let mut tokens = vec![];

    let ccs = cainome_cairo_serde();

    tokens.push(quote! {
        impl<#(#gen_args),* , #(#gen_args_rust),*> #ccs::CairoSerde for #entity_name<#(#gen_args),*>
        where
    });

    for (i, g) in gen_args.iter().enumerate() {
        let gr = &gen_args_rust[i];
        tokens.push(quote!(#g: #ccs::CairoSerde<RustType = #gr>,));
    }

    quote!(#(#tokens)*)
}

/// Expands the associated types lines for generic types.
pub fn rust_associated_type_gen_args(entity_name: &Ident, gen_args: &[Ident]) -> TokenStream2 {
    let gen_args_rust: Vec<Ident> = gen_args
        .iter()
        .map(|g| str_to_ident(format!("R{}", g).as_str()))
        .collect();

    quote!(type RustType = #entity_name<#(#gen_args_rust),*>;)
}

#[derive(Debug, PartialEq)]
enum SerdeHexType {
    None,
    Single,
    Tuple(usize),
    Vec,
}

impl SerdeHexType {
    pub fn is_none(&self) -> bool {
        matches!(self, SerdeHexType::None)
    }
}

/// Serde derive for hex serialization of struct member or enum variant.
/// In the case of tuples, all the elements will be serialized as hex.
pub fn serde_hex_derive(ty: &str) -> TokenStream2 {
    let serde_single = format!("{}::serialize_as_hex", cainome_cairo_serde_path());
    let serde_vec = format!("{}::serialize_as_hex_vec", cainome_cairo_serde_path());
    let serde_tuple_2 = format!("{}::serialize_as_hex_t2", cainome_cairo_serde_path());
    let serde_tuple_3 = format!("{}::serialize_as_hex_t3", cainome_cairo_serde_path());

    let serde_hex = is_serde_hex_int(ty);

    match serde_hex {
        SerdeHexType::None => quote!(),
        SerdeHexType::Single => quote! {
            #[serde(serialize_with = #serde_single)]
        },
        SerdeHexType::Tuple(2) => quote! {
            #[serde(serialize_with = #serde_tuple_2)]
        },
        SerdeHexType::Tuple(3) => quote! {
            #[serde(serialize_with = #serde_tuple_3)]
        },
        SerdeHexType::Vec => quote! {
            #[serde(serialize_with = #serde_vec)]
        },
        _ => panic!("Unsupported type {} for serde_hex", ty),
    }
}

/// To simplify the serde interop with client in javascript,
/// we use the hex format for all the types greater than u32.
/// IEEE 754 standard for floating-point arithmetic,
/// which can safely represent integers up to 2^53 - 1, which is what Javascript uses.
///
/// This function returns the number of type that require serde_hex serialization.
/// The type might be a single integer type, or a tuple of integer types.
#[inline]
fn is_serde_hex_int(ty: &str) -> SerdeHexType {
    let tuple = is_serde_hex_tuple(ty);
    if !tuple.is_none() {
        return tuple;
    }

    let vec = is_serde_hex_vec(ty);
    if !vec.is_none() {
        return vec;
    }

    if ty == "u128" || ty == "u64" || ty == "i128" || ty == "i64" {
        return SerdeHexType::Single;
    }

    SerdeHexType::None
}

/// Checks if the type is a tuple of integers that should be serialized as hex.
fn is_serde_hex_tuple(ty: &str) -> SerdeHexType {
    if ty.starts_with('(') && ty.ends_with(')') {
        let elements: Vec<&str> = ty[1..ty.len() - 1].split(',').collect();

        let has_hex_int = elements
            .iter()
            .any(|t| !is_serde_hex_int(t.trim()).is_none());

        if has_hex_int {
            return SerdeHexType::Tuple(elements.len());
        } else {
            return SerdeHexType::None;
        }
    }

    SerdeHexType::None
}

/// Checks if the type is a vector of integers that should be serialized as hex.
fn is_serde_hex_vec(ty: &str) -> SerdeHexType {
    if ty.starts_with("Vec<") && ty.ends_with('>') {
        let inner_type = &ty[4..ty.len() - 1];

        if !is_serde_hex_int(inner_type).is_none() {
            return SerdeHexType::Vec;
        } else {
            return SerdeHexType::None;
        }
    }

    SerdeHexType::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_serde_hex_int() {
        assert_eq!(is_serde_hex_int("u128"), SerdeHexType::Single);
        assert_eq!(is_serde_hex_int("u64"), SerdeHexType::Single);
        assert_eq!(is_serde_hex_int("i128"), SerdeHexType::Single);
        assert_eq!(is_serde_hex_int("i64"), SerdeHexType::Single);
        assert_eq!(is_serde_hex_int("u32"), SerdeHexType::None);
        assert_eq!(is_serde_hex_int("u16"), SerdeHexType::None);
        assert_eq!(is_serde_hex_int("u8"), SerdeHexType::None);
        assert_eq!(is_serde_hex_int("i32"), SerdeHexType::None);
        assert_eq!(is_serde_hex_int("i16"), SerdeHexType::None);
        assert_eq!(is_serde_hex_int("i8"), SerdeHexType::None);
    }

    #[test]
    fn test_is_serde_hex_tuple() {
        assert_eq!(is_serde_hex_tuple("(u32, u64)"), SerdeHexType::Tuple(2));
        assert_eq!(
            is_serde_hex_tuple("(u128, u64, i128)"),
            SerdeHexType::Tuple(3)
        );
        assert_eq!(is_serde_hex_tuple("(felt252, u32)"), SerdeHexType::None);
    }

    #[test]
    fn test_is_serde_hex_vec() {
        assert_eq!(is_serde_hex_vec("Vec<u128>"), SerdeHexType::Vec);
        assert_eq!(is_serde_hex_vec("Vec<i64>"), SerdeHexType::Vec);
        assert_eq!(is_serde_hex_vec("Vec<u32>"), SerdeHexType::None);
        assert_eq!(is_serde_hex_vec("Vec<MyStruct>"), SerdeHexType::None);
    }
}
