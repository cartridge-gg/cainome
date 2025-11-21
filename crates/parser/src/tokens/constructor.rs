use std::rc::Rc;

use crate::{tokens::Token, CainomeResult};

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorInner {
    pub name: String,
    pub token: Rc<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constructor {
    pub type_path: String,
    pub inputs: Vec<ConstructorInner>,
}

impl Constructor {
    pub fn new(type_path: &str) -> CainomeResult<Self> {
        return Ok(Self {
            type_path: type_path.to_string(),
            inputs: vec![],
        });
    }
}
