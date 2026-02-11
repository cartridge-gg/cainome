use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use cainome_parser::tokens::{NamedToken, Token};

use crate::expand::{generic_resolver::ResolveResult, types::CairoToRust, utils, ExpansionContext};

pub fn resolve_generics(
    full_type_name: &str,
    field: &NamedToken,
    fields_to_generics: &HashMap<String, HashSet<String>>,
    ctx: &ExpansionContext,
) -> String {
    let original_type = (&*field.token.borrow()).to_rust_type_path(ctx);

    // Calculate default value for field to assign to generic_type (in case resolver won't work)
    let default_generic_type = fields_to_generics
        // Check if generic candidates exist for the field
        .get(&field.name)
        .unwrap_or(&HashSet::new())
        .iter()
        .next()
        .cloned();

    tracing::trace!("Resolver itself: {:?}", ctx.generic_resolver);

    let resolved_result = ctx
        .generic_resolver
        .resolve_generic_member(full_type_name, field, ctx);

    tracing::trace!(
        "Resolved generic for field {} of type {}: {:?}",
        field.name,
        full_type_name,
        resolved_result
    );

    match resolved_result {
        // If generic was resolved, use it's result or default original type
        ResolveResult::Resolved(path) => path.unwrap_or(original_type),
        // If generic could not result use default naive algorythm
        ResolveResult::Unresolved => default_generic_type.unwrap_or(original_type),
    }
}

pub fn get_generic_args_fields(input: &Vec<(String, Rc<RefCell<Token>>)>) -> Vec<syn::Ident> {
    input
        .iter()
        .map(|(name, _)| utils::str_to_ident(name))
        .collect::<Vec<_>>()
}
