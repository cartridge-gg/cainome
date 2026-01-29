use std::{cell::RefCell, rc::Rc};

use crate::tokens::Token;

#[derive(Debug, Clone, PartialEq)]
pub struct NamedToken {
    pub name: String,
    pub token: Rc<RefCell<Token>>,
}

impl NamedToken {
    pub fn new(name: &str, token: Rc<RefCell<Token>>) -> Self {
        Self {
            name: name.to_string(),
            token,
        }
    }
}
