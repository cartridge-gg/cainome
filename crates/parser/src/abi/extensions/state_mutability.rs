use starknet::core::types::contract::StateMutability;

impl From<StateMutability> for crate::tokens::StateMutability {
    fn from(value: StateMutability) -> Self {
        match value {
            StateMutability::External => crate::tokens::StateMutability::External,
            StateMutability::View => crate::tokens::StateMutability::View,
        }
    }
}
