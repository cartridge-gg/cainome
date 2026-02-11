use std::{cell::RefCell, rc::Rc};

use crate::{
    abi::registry::TypeRegistry,
    tokens::{genericity, utils::escape_rust_keywords, NamedToken, Token},
    CainomeResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub type_path: String,
    pub fields: Vec<NamedToken>,
    pub generic_args: Vec<(String, Rc<RefCell<Token>>)>,
}

impl Struct {
    pub fn new(type_path: &str, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let Ok(generic_args_with_types) = generic_args
            .into_iter()
            .map(|(name, path)| registry.get(&path).map(|t| (name, t)))
            .collect::<Result<Vec<_>, _>>()
        else {
            unreachable!("Generic args should be registered in the registry");
        };

        Ok(Self {
            // TODO: need to remove the code that strips generics from type_path then.
            type_path: genericity::type_path_no_generic(&type_path),
            generic_args: generic_args_with_types,
            fields: vec![],
        })
    }

    pub fn with_field(mut self, name: &str, token: Rc<RefCell<Token>>) -> Self {
        self.fields.push(NamedToken {
            name: name.to_string(),
            token: Rc::clone(&token),
        });
        self
    }

    pub fn with_fields(mut self, fields: Vec<NamedToken>) -> Self {
        for field in fields {
            self = self.with_field(&field.name, Rc::clone(&field.token));
        }
        self
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn is_generic(&self) -> bool {
        !self.generic_args.is_empty()
    }
}
