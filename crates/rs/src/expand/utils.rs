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

/// To simplify the serde interop with client in javascript,
/// we use the hex format for all the types greater than u32.
/// IEEE 754 standard for floating-point arithmetic,
/// which can safely represent integers up to 2^53 - 1, which is what Javascript uses.
///
/// This function returns the number of type that require serde_hex serialization.
/// The type might be a single integer type, or a tuple of integer types.
#[inline]
fn is_serde_hex_int(ty: &str) -> usize {
    fn parse_tuple(ty: &str) -> (bool, usize) {
        if ty.starts_with('(') && ty.ends_with(')') {
            let elements: Vec<&str> = ty[1..ty.len() - 1].split(',').collect();
            let is_hex_int = elements.iter().any(|t| is_serde_hex_int(t.trim()) > 0);
            return (is_hex_int, elements.len());
        }

        (false, 0)
    }

    let (is_tuple, n) = parse_tuple(ty);
    if is_tuple {
        return n;
    }

    if ty == "u128" || ty == "u64" || ty == "i128" || ty == "i64" {
        return 1;
    }

    0
}

/// Serde derive for hex serialization of struct member or enum variant.
/// In the case of tuples, all the elements will be serialized as hex.
pub fn serde_hex_derive(ty: &str) -> TokenStream2 {
    let serde_path_1 = format!("{}::serialize_as_hex", cainome_cairo_serde_path());
    let serde_path_2 = format!("{}::serialize_as_hex_t2", cainome_cairo_serde_path());
    let serde_path_3 = format!("{}::serialize_as_hex_t3", cainome_cairo_serde_path());

    let n_serde_hex = is_serde_hex_int(ty);

    match n_serde_hex {
        0 => quote!(),
        1 => quote! {
            #[serde(serialize_with = #serde_path_1)]
        },
        2 => quote! {
            #[serde(serialize_with = #serde_path_2)]
        },
        3 => quote! {
            #[serde(serialize_with = #serde_path_3)]
        },
        _ => panic!("Unsupported type {} for serde_hex", ty),
    }
}
