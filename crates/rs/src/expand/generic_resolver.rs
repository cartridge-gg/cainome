use std::{collections::HashMap, rc::Rc};

use cainome_parser::tokens::NamedToken;

use crate::expand::ExpansionContext;

pub trait GenericResolver: std::fmt::Debug {
    fn resolve_generic_member(
        &self,
        type_path: &str,
        field: &NamedToken,
        ctx: &ExpansionContext,
    ) -> GenericResolveResult;
}

#[derive(Debug, Clone, Default)]
pub struct DefaultGenericResolver;

#[derive(Debug)]
pub enum GenericResolveResult {
    Resolved(Option<String>),
    Unresolved,
}

impl GenericResolver for DefaultGenericResolver {
    fn resolve_generic_member(
        &self,
        _type_path: &str,
        _field: &NamedToken,
        _ctx: &ExpansionContext,
    ) -> GenericResolveResult {
        GenericResolveResult::Unresolved
    }
}

#[derive(Debug, Clone, Default)]
pub struct GenericResolverFromMapping {
    mappings: HashMap<(String, String), String>,
}

impl GenericResolver for GenericResolverFromMapping {
    fn resolve_generic_member(
        &self,
        type_path: &str,
        field: &NamedToken,
        _ctx: &ExpansionContext,
    ) -> GenericResolveResult {
        let key = &(type_path.to_string(), field.name.clone());
        let val = self.mappings.get(key).cloned();

        tracing::trace!(
            "Resolving generic member: type_path={}, field={}, resolved={:?}",
            type_path,
            field.name,
            val
        );

        GenericResolveResult::Resolved(val)
    }
}

impl GenericResolverFromMapping {
    pub fn new<T>(mappings: Vec<(T, T, T)>) -> Rc<Self>
    where
        T: AsRef<str>,
    {
        let mappings = mappings
            .into_iter()
            .map(|(type_path, field_name, generic_arg)| {
                (
                    (
                        type_path.as_ref().to_owned(),
                        field_name.as_ref().to_owned(),
                    ),
                    generic_arg.as_ref().to_owned(),
                )
            })
            .collect();
        Rc::new(Self { mappings })
    }
}
