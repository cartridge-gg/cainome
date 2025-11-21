use std::rc::Rc;

use crate::{
    abi::registry::TypeRegistry,
    tokens::{genericity, utils, Token},
    CainomeResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct EnumInner {
    pub name: String,
    pub token: Rc<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub type_path: String,
    pub variants: Vec<EnumInner>,
    pub generic_args: Vec<(String, Rc<Token>)>,
    pub alias: Option<String>,
}

impl Enum {
    pub fn new(type_path: String, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(&type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let generic_args_with_types: Vec<(String, Rc<Token>)> = generic_args
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

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn type_name(&self) -> String {
        // TODO: need to opti that with regex?
        utils::extract_type_path_with_depth(&self.type_path_no_generic(), 0)
    }

    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        Err(crate::Error::ParsingFailed("asd".to_string()))
        // We want to keep the path with generic for the generic resolution.
        // Ok(Self {
        //     type_path: type_path.to_string(),
        //     generic_args: generic_args,
        //     variants: vec![],
        //     alias: None,
        // })
    }
}
