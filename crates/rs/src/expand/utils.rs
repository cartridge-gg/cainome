//! Utils function for expansion.
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, Type};

///
pub fn str_to_ident(str_in: &str) -> Ident {
    Ident::new(str_in, proc_macro2::Span::call_site())
}

///
pub fn str_to_type(str_in: &str) -> Type {
    syn::parse_str(str_in).unwrap_or_else(|_| panic!("Can't convert {} to syn::Type", str_in))
}

///
// pub fn str_to_litstr(str_in: &str) -> LitStr {
//     LitStr::new(str_in, proc_macro2::Span::call_site())
// }

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
