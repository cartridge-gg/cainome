//! CairoSerde implementation for starknet types.
//!
//! They are alf `Felt` under the hood.
use crate::{CairoSerde, Error, Result};
use starknet::core::types::Felt;

/// ContractAddress.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct ContractAddress(pub Felt);

impl From<Felt> for ContractAddress {
    fn from(item: Felt) -> Self {
        Self(item)
    }
}

impl From<ContractAddress> for Felt {
    fn from(item: ContractAddress) -> Self {
        item.0
    }
}

impl CairoSerde for ContractAddress {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
        Felt::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize a ContractAddress: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        Ok(ContractAddress(Felt::cairo_deserialize(felts, offset)?))
    }
}

/// ClassHash.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct ClassHash(pub Felt);

impl From<Felt> for ClassHash {
    fn from(item: Felt) -> Self {
        Self(item)
    }
}

impl From<ClassHash> for Felt {
    fn from(item: ClassHash) -> Self {
        item.0
    }
}

impl CairoSerde for ClassHash {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
        Felt::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize a ClassHash: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        Ok(ClassHash(Felt::cairo_deserialize(felts, offset)?))
    }
}

/// EthAddress.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct EthAddress(pub Felt);

impl From<Felt> for EthAddress {
    fn from(item: Felt) -> Self {
        Self(item)
    }
}

impl From<EthAddress> for Felt {
    fn from(item: EthAddress) -> Self {
        item.0
    }
}

impl CairoSerde for EthAddress {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
        Felt::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize an EthAddress: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        Ok(EthAddress(Felt::cairo_deserialize(felts, offset)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_address_cairo_serialize() {
        let contract_address = ContractAddress(Felt::from(1_u32));
        let felts = ContractAddress::cairo_serialize(&contract_address);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(1_u32));
    }

    #[test]
    fn test_contract_address_cairo_deserialize() {
        let felts = vec![Felt::from(1_u32)];
        let contract_address = ContractAddress::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(contract_address, ContractAddress(Felt::from(1_u32)))
    }

    #[test]
    fn test_class_hash_cairo_serialize() {
        let class_hash = ClassHash(Felt::from(1_u32));
        let felts = ClassHash::cairo_serialize(&class_hash);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(1_u32));
    }

    #[test]
    fn test_class_hash_cairo_deserialize() {
        let felts = vec![Felt::from(1_u32)];
        let class_hash = ClassHash::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(class_hash, ClassHash(Felt::from(1_u32)))
    }

    #[test]
    fn test_eth_address_cairo_serialize() {
        let eth_address = EthAddress(Felt::from(1_u32));
        let felts = EthAddress::cairo_serialize(&eth_address);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(1_u32));
    }

    #[test]
    fn test_eth_address_cairo_deserialize() {
        let felts = vec![Felt::from(1_u32)];
        let eth_address = EthAddress::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(eth_address, EthAddress(Felt::from(1_u32)))
    }

    #[test]
    fn test_contract_address_from() {
        let contract_address = ContractAddress::from(Felt::from(1_u32));
        assert_eq!(contract_address, ContractAddress(Felt::from(1_u32)))
    }

    #[test]
    fn test_class_hash_from() {
        let class_hash = ClassHash::from(Felt::from(1_u32));
        assert_eq!(class_hash, ClassHash(Felt::from(1_u32)))
    }

    #[test]
    fn test_eth_address_from() {
        let eth_address = EthAddress::from(Felt::from(1_u32));
        assert_eq!(eth_address, EthAddress(Felt::from(1_u32)))
    }
}
