use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    abi::registry::TypeRegistry,
    tokens::{genericity, utils, NamedToken, Token},
    CainomeResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub type_path: String,
    pub variants: Vec<NamedToken>,
    pub generic_args: Vec<(String, Rc<RefCell<Token>>)>,
    pub alias: Option<String>,
}

impl Enum {
    pub fn new(type_path: &str, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let generic_args_with_types: Vec<(String, Rc<RefCell<Token>>)> = generic_args
            .into_iter()
            .map(|(name, path)| (name, registry.get(&path).unwrap()))
            .collect();

        return Ok(Self {
            type_path,
            generic_args: generic_args_with_types,
            variants: vec![],
            alias: None,
        });
    }

    pub fn with_variant(self, name: &str, token: Rc<RefCell<Token>>) -> Self {
        Self {
            variants: {
                let mut v = self.variants;
                v.push(NamedToken {
                    name: name.to_string(),
                    token,
                });
                v
            },
            ..self
        }
    }

    pub fn with_variants(self, variants: Vec<NamedToken>) -> Self {
        Self { variants, ..self }
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }
}
