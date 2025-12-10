//! Function tokens.
use std::{cell::RefCell, rc::Rc};

use convert_case::{Case, Casing};

use crate::tokens::NamedToken;

use super::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum StateMutability {
    External,
    View,
}

#[derive(Debug)]
pub enum FunctionOutputKind {
    NoOutput,
    Cairo1,
    Cairo0,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub state_mutability: StateMutability,
    pub inputs: Vec<NamedToken>,
    pub outputs: Vec<Rc<RefCell<Token>>>,
    // Only cairo0 has named outputs.
    pub named_outputs: Vec<NamedToken>,
}

impl Function {
    pub fn new(name: &str, state_mutability: StateMutability) -> Self {
        Self {
            name: name.to_string(),
            state_mutability,
            inputs: vec![],
            outputs: vec![],
            named_outputs: vec![],
        }
    }

    pub fn get_output_kind(&self) -> FunctionOutputKind {
        match (self.outputs.is_empty(), self.named_outputs.is_empty()) {
            (true, true) => FunctionOutputKind::NoOutput,
            (false, true) => FunctionOutputKind::Cairo1,
            (true, false) => FunctionOutputKind::Cairo0,
            (false, false) => panic!("Function's outputs and named outputs are exclusive!"),
        }
    }

    pub fn get_cairo0_output_name(&self) -> String {
        format!(
            "{}Output",
            self.name.from_case(Case::Snake).to_case(Case::Pascal)
        )
    }
}
