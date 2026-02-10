use cainome_parser::tokens::NamedToken;

use crate::expand::ExpansionContext;

pub trait GenericResolver {
    fn resolve_generic_member(
        &self,
        type_path: &str,
        field: &NamedToken,
        ctx: &ExpansionContext,
    ) -> Option<String>;
}

pub struct DefaultGenericResolver;

impl DefaultGenericResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl GenericResolver for DefaultGenericResolver {
    fn resolve_generic_member(
        &self,
        _type_path: &str,
        _field: &NamedToken,
        _ctx: &ExpansionContext,
    ) -> Option<String> {
        None
    }
}
