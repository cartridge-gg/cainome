use std::{cell::RefCell, rc::Rc};

use crate::{tokens::Token, CainomeResult};

#[derive(Debug, Clone, PartialEq)]
pub struct Interface {
    pub type_path: String,
    // That is not great, as actually only Token::Function is allowed
    pub functions: Vec<Rc<RefCell<Token>>>,
}

impl Interface {
    pub fn new(type_path: &str) -> CainomeResult<Self> {
        return Ok(Self {
            type_path: type_path.to_string(),
            functions: vec![],
        });
    }

    pub fn type_name(&self) -> String {
        self.type_path.clone()
    }
}
