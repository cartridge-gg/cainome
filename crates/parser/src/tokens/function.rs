//! Function tokens.
use convert_case::{Case, Casing};

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
    pub inputs: Vec<(String, Token)>,
    pub outputs: Vec<Token>,
    // Only cairo0 has named outputs.
    pub named_outputs: Vec<(String, Token)>,
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

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        for (_, ref mut t) in &mut self.inputs {
            if let Token::Composite(ref mut c) = t {
                c.apply_alias(type_path, alias);
            }
        }

        for ref mut t in &mut self.outputs {
            if let Token::Composite(ref mut c) = t {
                c.apply_alias(type_path, alias);
            }
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
