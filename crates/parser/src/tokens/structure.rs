use std::{cell::RefCell, rc::Rc};

use crate::{
    abi::registry::TypeRegistry,
    tokens::{
        genericity,
        utils::{self, escape_rust_keywords},
        NamedToken, Token,
    },
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

        let generic_args_with_types: Vec<(String, Rc<RefCell<Token>>)> = generic_args
            .into_iter()
            .map(|(name, path)| (name, registry.get(&path).unwrap()))
            .collect();

        Ok(Self {
            type_path,
            generic_args: generic_args_with_types,
            fields: vec![],
        })
    }

    pub fn with_field(mut self, name: &str, token: Rc<RefCell<Token>>) -> Self {
        self.fields.push(NamedToken {
            name: name.to_string(),
            token: token,
        });
        self
    }

    pub fn with_fields(mut self, fields: Vec<NamedToken>) -> Self {
        self.fields.extend(fields);
        self
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }
}
