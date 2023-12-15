use super::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum StateMutability {
    External,
    View,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub state_mutability: StateMutability,
    pub inputs: Vec<(String, Token)>,
    pub outputs: Vec<Token>,
}

impl Function {
    pub fn new(name: &str, state_mutability: StateMutability) -> Self {
        Self {
            name: name.to_string(),
            state_mutability,
            inputs: vec![],
            outputs: vec![],
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
}
