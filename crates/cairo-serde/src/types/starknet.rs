//! CairoSerde implementation for starknet types.
//!
//! They are alf `FieldElement` under the hood.
use crate::{CairoSerde, Error, Result};
use starknet::core::types::FieldElement;

/// ContractAddress.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct ContractAddress(pub FieldElement);

impl From<FieldElement> for ContractAddress {
    fn from(item: FieldElement) -> Self {
        Self(item)
    }
}

impl From<ContractAddress> for FieldElement {
    fn from(item: ContractAddress) -> Self {
        item.0
    }
}

impl CairoSerde for ContractAddress {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        FieldElement::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize a ContractAddress: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        Ok(ContractAddress(FieldElement::cairo_deserialize(
            felts, offset,
        )?))
    }
}

/// ClassHash.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct ClassHash(pub FieldElement);

impl From<FieldElement> for ClassHash {
    fn from(item: FieldElement) -> Self {
        Self(item)
    }
}

impl From<ClassHash> for FieldElement {
    fn from(item: ClassHash) -> Self {
        item.0
    }
}

impl CairoSerde for ClassHash {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        FieldElement::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize a ClassHash: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        Ok(ClassHash(FieldElement::cairo_deserialize(felts, offset)?))
    }
}

/// EthAddress.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EthAddress(pub FieldElement);

impl From<FieldElement> for EthAddress {
    fn from(item: FieldElement) -> Self {
        Self(item)
    }
}

impl From<EthAddress> for FieldElement {
    fn from(item: EthAddress) -> Self {
        item.0
    }
}

impl CairoSerde for EthAddress {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        FieldElement::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize an EthAddress: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        Ok(EthAddress(FieldElement::cairo_deserialize(felts, offset)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_address_cairo_serialize() {
        let contract_address = ContractAddress(FieldElement::from(1_u32));
        let felts = ContractAddress::cairo_serialize(&contract_address);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::from(1_u32));
    }

    #[test]
    fn test_contract_address_cairo_deserialize() {
        let felts = vec![FieldElement::from(1_u32)];
        let contract_address = ContractAddress::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(contract_address, ContractAddress(FieldElement::from(1_u32)))
    }

    #[test]
    fn test_class_hash_cairo_serialize() {
        let class_hash = ClassHash(FieldElement::from(1_u32));
        let felts = ClassHash::cairo_serialize(&class_hash);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::from(1_u32));
    }

    #[test]
    fn test_class_hash_cairo_deserialize() {
        let felts = vec![FieldElement::from(1_u32)];
        let class_hash = ClassHash::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(class_hash, ClassHash(FieldElement::from(1_u32)))
    }

    #[test]
    fn test_eth_address_cairo_serialize() {
        let eth_address = EthAddress(FieldElement::from(1_u32));
        let felts = EthAddress::cairo_serialize(&eth_address);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::from(1_u32));
    }

    #[test]
    fn test_eth_address_cairo_deserialize() {
        let felts = vec![FieldElement::from(1_u32)];
        let eth_address = EthAddress::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(eth_address, EthAddress(FieldElement::from(1_u32)))
    }

    #[test]
    fn test_contract_address_from() {
        let contract_address = ContractAddress::from(FieldElement::from(1_u32));
        assert_eq!(contract_address, ContractAddress(FieldElement::from(1_u32)))
    }

    #[test]
    fn test_class_hash_from() {
        let class_hash = ClassHash::from(FieldElement::from(1_u32));
        assert_eq!(class_hash, ClassHash(FieldElement::from(1_u32)))
    }

    #[test]
    fn test_eth_address_from() {
        let eth_address = EthAddress::from(FieldElement::from(1_u32));
        assert_eq!(eth_address, EthAddress(FieldElement::from(1_u32)))
    }
}
