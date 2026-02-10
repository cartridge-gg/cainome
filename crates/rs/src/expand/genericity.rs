use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use cainome_parser::tokens::{NamedToken, Token};

use crate::expand::{types::CairoToRust, utils, ExpansionContext};

pub fn resolve_generics(
    type_name: &str,
    field: &NamedToken,
    fields_to_generics: &HashMap<String, HashSet<String>>,
    ctx: &ExpansionContext,
) -> String {
    // Calculate default value for field to assign to generic_type (in case resolver won't work)
    let default_generic_type = fields_to_generics
        // Check if generic candidates exist for the field
        .get(&field.name)
        .unwrap_or(&HashSet::new())
        .iter()
        .next()
        .cloned()
        // if not use type from ABI
        .unwrap_or((&*field.token.borrow()).to_rust_type_path(ctx));

    ctx.generic_resolver
        .resolve_generic_member(type_name, field, ctx)
        .unwrap_or(default_generic_type)
}

fn get_generic_args_fields(input: Vec<(String, Rc<RefCell<Token>>)>) -> Vec<syn::Ident> {
    input
        .iter()
        .map(|(name, _)| utils::str_to_ident(name))
        .collect::<Vec<_>>()
}
