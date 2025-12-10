use starknet::core::types::{contract::StateMutability, FunctionStateMutability};

impl From<StateMutability> for crate::tokens::StateMutability {
    fn from(value: StateMutability) -> Self {
        match value {
            StateMutability::External => crate::tokens::StateMutability::External,
            StateMutability::View => crate::tokens::StateMutability::View,
        }
    }
}

impl From<Option<FunctionStateMutability>> for crate::tokens::StateMutability {
    fn from(value: Option<FunctionStateMutability>) -> Self {
        match value {
            Option::Some(FunctionStateMutability::View) => crate::tokens::StateMutability::View,
            _ => crate::tokens::StateMutability::External,
        }
    }
}
