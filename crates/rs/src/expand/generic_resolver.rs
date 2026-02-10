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

#[derive(Debug, Clone, Default)]
pub struct DefaultGenericResolver;

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
