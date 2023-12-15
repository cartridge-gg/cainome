use cainome_parser::tokens::{Composite, Token};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use crate::expand::utils;
use crate::expand::types::CairoToRust;

pub struct CairoEvent;

impl CairoEvent {
    pub fn expand(enums: &[Token], structs: &[Token]) -> TokenStream2 {
        // TODO:
        // For each enum in enums -> check if it's an event.
        // if yes ->
        // 1. impl a function to retrieve the selector + string name of the event.
        // 2. impl `TryFrom` EmittedEvent. Need to take in account the new flat keyword.
        //    - if nested => the selector is the name of the enum variant.
        //    - if nested and the type it points to is also an enum => first selector is
        //      the name of the variant of the current enum, and then we've to check
        //      recursively until the event type is a struct and not an enum.
        //    - if it's flat, we just take the name of the current variant.
        quote!()
    }
}
