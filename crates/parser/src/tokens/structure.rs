use crate::{
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
    pub token: Token,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub type_path: String,
    pub fields: Vec<StructInner>,
    pub generic_args: Vec<(String, Token)>,
    pub alias: Option<String>,
}

impl Struct {
    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn type_name(&self) -> String {
        // TODO: need to opti that with regex?
        utils::extract_type_path_with_depth(&self.type_path_no_generic(), 0)
    }

    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        let type_path = escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        // We want to keep the path with generic for the generic resolution.
        Ok(Self {
            type_path: type_path.to_string(),
            generic_args: generic_args,
            fields: vec![],
            alias: None,
        })
    }
}
