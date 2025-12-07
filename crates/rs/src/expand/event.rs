use cainome_parser::tokens::Event;
use proc_macro2::TokenStream;

use crate::expand::Expandable;

impl Expandable for Event {
    fn expand(&self, expansion_context: &super::ExpansionContext) -> Vec<TokenStream> {
        todo!()
    }
}
