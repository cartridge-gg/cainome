use std::cell::RefCell;
use std::rc::Rc;

use super::constants::{CAIRO_COMPOSITE_BUILTINS, CAIRO_GENERIC_BUILTINS};
use super::genericity;
use super::Token;

use crate::abi::registry::TypeRegistry;
use crate::tokens::utils;
use crate::CainomeResult;

#[derive(Debug, Clone, PartialEq)]
pub struct EventInner {
    pub name: String,
    pub token: Rc<RefCell<Token>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub type_path: String,
    pub keys: Vec<EventInner>,
    pub data: Vec<EventInner>,
    pub nested: Vec<EventInner>,
    pub flat: Vec<EventInner>,
    pub generic_args: Vec<(String, Rc<RefCell<Token>>)>,
}

impl Event {
    pub fn new(type_path: String, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(&type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let generic_args_with_types: Vec<(String, Rc<RefCell<Token>>)> = generic_args
            .into_iter()
            .map(|(name, path)| (name, registry.get(&path).unwrap()))
            .collect();

        return Ok(Self {
            type_path,
            generic_args: generic_args_with_types,
            keys: vec![],
            data: vec![],
            nested: vec![],
            flat: vec![],
        });
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn is_generic(&self) -> bool {
        !self.generic_args.is_empty()
    }

    pub fn type_name(&self) -> String {
        // TODO: need to opti that with regex?
        utils::extract_type_path_with_depth(&self.type_path_no_generic(), 0)
    }
}
