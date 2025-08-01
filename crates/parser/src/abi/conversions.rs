use starknet::core::types::contract::{
    legacy::{RawLegacyEvent, RawLegacyStruct},
    AbiEnum, AbiEventEnum, AbiEventStruct, AbiStruct, EventFieldKind,
    StateMutability as StarknetStateMutability,
};

use crate::tokens::{CompositeInner, CompositeInnerKind, CompositeType, StateMutability, Token};
use crate::Error;

impl From<StarknetStateMutability> for StateMutability {
    fn from(value: StarknetStateMutability) -> Self {
        match value {
            StarknetStateMutability::External => StateMutability::External,
            StarknetStateMutability::View => StateMutability::View,
        }
    }
}

impl From<EventFieldKind> for CompositeInnerKind {
    fn from(value: EventFieldKind) -> Self {
        match value {
            EventFieldKind::Key => CompositeInnerKind::Key,
            EventFieldKind::Data => CompositeInnerKind::Data,
            EventFieldKind::Nested => CompositeInnerKind::Nested,
            EventFieldKind::Flat => CompositeInnerKind::Flat,
        }
    }
}

impl TryFrom<&AbiStruct> for Token {
    type Error = Error;

    fn try_from(value: &AbiStruct) -> Result<Self, Self::Error> {
        let mut t = Token::parse(&value.name)?;

        if let Token::Composite(ref mut c) = t {
            c.r#type = CompositeType::Struct;

            for (i, m) in value.members.iter().enumerate() {
                c.inners.push(CompositeInner {
                    index: i,
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type).unwrap(),
                    kind: CompositeInnerKind::NotUsed,
                });
            }

            Ok(t)
        } else {
            Err(Error::ParsingFailed(format!(
                "AbiStruct is expected to be a Composite token, got `{:?}`",
                value,
            )))
        }
    }
}

impl TryFrom<&AbiEnum> for Token {
    type Error = Error;

    fn try_from(value: &AbiEnum) -> Result<Self, Self::Error> {
        let mut t = Token::parse(&value.name)?;

        if t.type_name() == "option" {
            return Ok(t);
        }

        if t.type_name() == "result" {
            return Ok(t);
        }

        if let Token::Composite(ref mut c) = t {
            c.r#type = CompositeType::Enum;

            for (i, v) in value.variants.iter().enumerate() {
                // Determine the kind based on whether the variant has data
                let kind = if v.r#type == "()" {
                    CompositeInnerKind::NotUsed
                } else {
                    CompositeInnerKind::Data
                };

                c.inners.push(CompositeInner {
                    index: i,
                    name: v.name.clone(),
                    token: Token::parse(&v.r#type).unwrap(),
                    kind,
                });
            }

            Ok(t)
        } else {
            Err(Error::ParsingFailed(format!(
                "AbiEnum is expected to be a Composite token, got `{:?}`",
                value,
            )))
        }
    }
}

impl TryFrom<&AbiEventStruct> for Token {
    type Error = Error;

    fn try_from(value: &AbiEventStruct) -> Result<Self, Self::Error> {
        let mut t = Token::parse(&value.name)?;

        if let Token::Composite(ref mut c) = t {
            c.r#type = CompositeType::Struct;
            c.is_event = true;

            for (i, m) in value.members.iter().enumerate() {
                c.inners.push(CompositeInner {
                    index: i,
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type).unwrap(),
                    kind: m.kind.clone().into(),
                });
            }

            Ok(t)
        } else {
            Err(Error::ParsingFailed(format!(
                "AbiEventStruct is expected to be a Composite token, got `{:?}`",
                value,
            )))
        }
    }
}

impl TryFrom<&AbiEventEnum> for Token {
    type Error = Error;

    fn try_from(value: &AbiEventEnum) -> Result<Self, Self::Error> {
        let mut t = Token::parse(&value.name)?;

        if let Token::Composite(ref mut c) = t {
            c.r#type = CompositeType::Enum;
            c.is_event = true;

            for (i, v) in value.variants.iter().enumerate() {
                c.inners.push(CompositeInner {
                    index: i,
                    name: v.name.clone(),
                    token: Token::parse(&v.r#type).unwrap(),
                    kind: v.kind.clone().into(),
                });
            }

            Ok(t)
        } else {
            Err(Error::ParsingFailed(format!(
                "AbiEventEnum is expected to be a Composite token, got `{:?}`",
                value,
            )))
        }
    }
}

impl TryFrom<&RawLegacyStruct> for Token {
    type Error = Error;

    fn try_from(value: &RawLegacyStruct) -> Result<Self, Self::Error> {
        let mut t = Token::parse(&value.name)?;

        if let Token::Composite(ref mut c) = t {
            c.r#type = CompositeType::Struct;

            for (i, m) in value.members.iter().enumerate() {
                c.inners.push(CompositeInner {
                    index: i,
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type).unwrap(),
                    kind: CompositeInnerKind::NotUsed,
                });
            }

            Ok(t)
        } else {
            Err(Error::ParsingFailed(format!(
                "RawLegacyStruct is expected to be a Composite token, got `{:?}`",
                value,
            )))
        }
    }
}

impl TryFrom<&RawLegacyEvent> for Token {
    type Error = Error;

    fn try_from(value: &RawLegacyEvent) -> Result<Self, Self::Error> {
        let mut t = Token::parse(&value.name)?;

        if let Token::Composite(ref mut c) = t {
            c.r#type = CompositeType::Struct;
            c.is_event = true;

            let mut i = 0;

            for m in value.data.iter() {
                c.inners.push(CompositeInner {
                    index: i,
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type).unwrap(),
                    kind: CompositeInnerKind::Data,
                });

                i += 1;
            }

            for m in value.keys.iter() {
                c.inners.push(CompositeInner {
                    index: i,
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type).unwrap(),
                    kind: CompositeInnerKind::Key,
                });

                i += 1;
            }

            Ok(t)
        } else {
            Err(Error::ParsingFailed(format!(
                "RawLegacyEvent is expected to be a Composite token, got `{:?}`",
                value,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::CompositeType;
    use crate::AbiParser;
    use std::collections::HashMap;

    #[test]
    fn test_enum_variant_composite_inner_kind() {
        // Test ABI with enum variants - some with data, some without
        let abi_json = r#"
        [
            {
                "type": "enum",
                "name": "test::TestEnum",
                "variants": [
                    {
                        "name": "VariantWithoutData",
                        "type": "()"
                    },
                    {
                        "name": "VariantWithFelt252",
                        "type": "core::felt252"
                    },
                    {
                        "name": "VariantWithTuple",
                        "type": "(core::felt252, core::integer::u32)"
                    }
                ]
            }
        ]
        "#;

        let result = AbiParser::tokens_from_abi_string(abi_json, &HashMap::new()).unwrap();

        assert_eq!(result.enums.len(), 1);
        let enum_composite = result.enums[0].to_composite().unwrap();

        assert_eq!(enum_composite.r#type, CompositeType::Enum);
        assert_eq!(enum_composite.inners.len(), 3);

        // Check that variant without data has NotUsed kind
        assert_eq!(enum_composite.inners[0].name, "VariantWithoutData");
        assert_eq!(enum_composite.inners[0].kind, CompositeInnerKind::NotUsed);

        // Check that variant with felt252 has Data kind
        assert_eq!(enum_composite.inners[1].name, "VariantWithFelt252");
        assert_eq!(enum_composite.inners[1].kind, CompositeInnerKind::Data);

        // Check that variant with tuple has Data kind
        assert_eq!(enum_composite.inners[2].name, "VariantWithTuple");
        assert_eq!(enum_composite.inners[2].kind, CompositeInnerKind::Data);
    }
}
