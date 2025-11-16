use std::{collections::HashMap, rc::Rc};

use starknet::core::types::contract::{
    legacy::{RawLegacyEvent, RawLegacyStruct},
    AbiConstructor, AbiEntry, AbiEnum, AbiEvent, AbiEventEnum, AbiEventStruct, AbiFunction,
    AbiStruct, EventFieldKind, StateMutability as StarknetStateMutability, TypedAbiEvent,
    UntypedAbiEvent,
};

use crate::tokens::{
    CompositeInnerKind, Enum, EnumInner, Event, EventInner, FuncInner, Function, StateMutability,
    Struct, StructInner, Token,
};
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

pub trait TokenConvertible: Sized {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error>;
}

impl TokenConvertible for &AbiStruct {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut structure = Struct::new(&self.name)?;

        for field in self.members.iter() {
            structure.fields.push(StructInner {
                name: field.name.clone(),
                token: Token::parse(&field.r#type).unwrap(),
            });
        }

        Ok(Token::Struct(structure))
    }
}

impl TokenConvertible for &AbiEnum {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut enumeration = Enum::new(&self.name)?;

        // if t.type_name() == "option" {
        //     return Ok(t);
        // }

        // if t.type_name() == "result" {
        //     return Ok(t);
        // }

        for v in self.variants.iter() {
            enumeration.variants.push(EnumInner {
                name: v.name.clone(),
                token: Token::parse(&v.r#type).unwrap(),
            });
        }

        Ok(Token::Enum(enumeration))
    }
}

impl TokenConvertible for &UntypedAbiEvent {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone())?;

        for m in self.inputs.iter() {
            event.data.push(EventInner {
                name: m.name.clone(),
                token: Token::parse(&m.r#type)?,
            })
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertible for &AbiEventStruct {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone())?;

        for m in self.members.iter() {
            // nested and falt could be omitted here I suppose.
            match m.kind {
                EventFieldKind::Key => event.keys.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
                EventFieldKind::Data => event.data.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
                EventFieldKind::Nested => event.nested.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
                EventFieldKind::Flat => event.flat.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
            }
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertible for &AbiEventEnum {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone())?;

        for m in self.variants.iter() {
            match m.kind {
                // key and data could be omitted here I suppose.
                EventFieldKind::Key => event.keys.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
                EventFieldKind::Data => event.data.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
                EventFieldKind::Nested => event.nested.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
                EventFieldKind::Flat => event.flat.push(EventInner {
                    name: m.name.clone(),
                    token: Token::parse(&m.r#type)?,
                }),
            }
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertible for &RawLegacyEvent {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone())?;

        for m in self.data.iter() {
            event.data.push(EventInner {
                name: m.name.clone(),
                token: Token::parse(&m.r#type)?,
            });
        }

        for m in self.keys.iter() {
            event.keys.push(EventInner {
                name: m.name.clone(),
                token: Token::parse(&m.r#type)?,
            });
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertible for &RawLegacyStruct {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut structure = Struct::new(&self.name)?;

        for field in self.members.iter() {
            structure.fields.push(StructInner {
                name: field.name.clone(),
                token: Token::parse(&field.r#type).unwrap(),
            });
        }

        Ok(Token::Struct(structure))
    }
}

impl TokenConvertible for &AbiFunction {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        let mut function = Function::new(&self.name, self.state_mutability.clone().into());

        for input in self.inputs.iter() {
            function.inputs.push(FuncInner {
                name: input.name.clone(),
                token: Token::parse(&input.r#type).unwrap(),
            });
        }

        for output in self.outputs.iter() {
            function.outputs.push(Token::parse(&output.r#type).unwrap());
        }

        Ok(Token::Function(function))
    }
}

impl TokenConvertible for AbiEntry {
    fn to_token(&self, registry: &mut HashMap<String, Rc<Token>>) -> Result<Token, Error> {
        match self {
            AbiEntry::Function(abi_function) => abi_function.to_token(registry),
            AbiEntry::Event(AbiEvent::Typed(TypedAbiEvent::Enum(abi_event))) => {
                abi_event.to_token(registry)
            }
            AbiEntry::Event(AbiEvent::Typed(TypedAbiEvent::Struct(abi_event))) => {
                abi_event.to_token(registry)
            }
            AbiEntry::Event(AbiEvent::Untyped(abi_event)) => abi_event.to_token(registry),
            AbiEntry::Struct(abi_struct) => abi_struct.to_token(registry),
            AbiEntry::Enum(abi_enum) => abi_enum.to_token(registry),
            AbiEntry::Constructor(abi_constructor) => todo!(),
            AbiEntry::Impl(abi_impl) => todo!(),
            AbiEntry::Interface(abi_interface) => todo!(),
            AbiEntry::L1Handler(abi_function) => abi_function.to_token(registry),
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
