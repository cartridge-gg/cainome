use std::{cell::RefCell, rc::Rc};

use crate::{
    abi::registry::{self, TypeRegistry},
    tokens::{
        genericity,
        utils::{self, escape_rust_keywords},
        Token,
    },
    CainomeResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct StructInner {
    pub name: String,
    pub token: Rc<RefCell<Token>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub type_path: String,
    pub fields: Vec<StructInner>,
    pub generic_args: Vec<(String, Rc<RefCell<Token>>)>,
}

impl Struct {
    pub fn new(type_path: String, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = escape_rust_keywords(&type_path);
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

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn type_name(&self) -> String {
        // TODO: need to opti that with regex?
        utils::extract_type_path_with_depth(&self.type_path_no_generic(), 0)
    }
}
