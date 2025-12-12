use std::cell::RefCell;
use std::rc::Rc;

use super::genericity;
use super::Token;

use crate::abi::registry::TypeRegistry;
use crate::tokens::{utils, NamedToken};
use crate::CainomeResult;

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    Enum,
    Struct,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub kind: EventKind, // TODO: split into different structs?
    pub type_path: String,

    // Only for kind == Struct
    pub keys: Vec<NamedToken>,
    pub data: Vec<NamedToken>,

    // Only for kind == Enum
    pub nested: Vec<NamedToken>,
    pub flat: Vec<NamedToken>,
    pub generic_args: Vec<(String, Rc<RefCell<Token>>)>,
}

impl Event {
    pub fn new(type_path: String, kind: EventKind, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(&type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let generic_args_with_types: Vec<(String, Rc<RefCell<Token>>)> = generic_args
            .into_iter()
            .map(|(name, path)| (name, registry.get(&path).unwrap()))
            .collect();

        return Ok(Self {
            kind: kind,
            type_path,
            generic_args: generic_args_with_types,
            keys: vec![],
            data: vec![],
            flat: vec![],
            nested: vec![],
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

    pub fn type_module(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
            .trim_end_matches("::")
            .to_string()
    }
}
