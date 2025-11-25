use std::rc::Rc;

use starknet::core::types::{
    contract::{
        legacy::{RawLegacyEvent, RawLegacyStruct},
        AbiEntry, AbiEnum, AbiEvent, AbiEventEnum, AbiEventStruct, AbiFunction, AbiInterface,
        AbiStruct, EventFieldKind, StateMutability as StarknetStateMutability, TypedAbiEvent,
        UntypedAbiEvent,
    },
    LegacyEventAbiEntry,
};

use crate::tokens::{
    CompositeInnerKind, Enum, EnumInner, Event, EventInner, FuncInner, Function, StateMutability,
    Struct, StructInner, Token,
};
use crate::{abi::registry::TypeRegistry, tokens::Interface, Error};

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

pub trait TokenConvertable: Sized {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error>;
}

pub trait Named {
    fn get_name(&self) -> String;
}

impl TokenConvertable for &AbiStruct {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut structure = Struct::new(self.name.clone(), &registry)?;

        for field in self.members.iter() {
            let token = registry.get(&field.r#type).unwrap();

            structure.fields.push(StructInner {
                name: field.name.clone(),
                token: token,
            });
        }

        Ok(Token::Struct(structure))
    }
}

impl TokenConvertable for &AbiEnum {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut enumeration = Enum::new(self.name.clone(), &registry)?;

        for field in self.variants.iter() {
            let token = registry.get(&field.r#type).unwrap();

            enumeration.variants.push(EnumInner {
                name: field.name.clone(),
                token: token,
            });
        }

        Ok(Token::Enum(enumeration))
    }
}

impl TokenConvertable for &UntypedAbiEvent {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone(), &registry)?;

        for field in self.inputs.iter() {
            let token = registry.get(&field.r#type)?;

            event.data.push(EventInner {
                name: field.name.clone(),
                token: token,
            });
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertable for &AbiEventStruct {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone(), &registry)?;

        for m in self.members.iter() {
            let token = registry.get(&m.r#type)?;

            let inner = EventInner {
                name: m.name.clone(),
                token: token,
            };

            // nested and falt could be omitted here I suppose.
            match m.kind {
                EventFieldKind::Key => event.keys.push(inner),
                EventFieldKind::Data => event.data.push(inner),
                EventFieldKind::Nested => event.nested.push(inner),
                EventFieldKind::Flat => event.flat.push(inner),
            }
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertable for &AbiEventEnum {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone(), &registry)?;

        for m in self.variants.iter() {
            let token = registry.get(&m.r#type)?;

            let inner = EventInner {
                name: m.name.clone(),
                token: token,
            };

            match m.kind {
                // key and data could be omitted here I suppose.
                EventFieldKind::Key => event.keys.push(inner),
                EventFieldKind::Data => event.data.push(inner),
                EventFieldKind::Nested => event.nested.push(inner),
                EventFieldKind::Flat => event.flat.push(inner),
            }
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertable for &RawLegacyEvent {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut event = Event::new(self.name.clone(), &registry)?;

        for m in self.data.iter() {
            let token = registry.get(&m.r#type)?;

            event.data.push(EventInner {
                name: m.name.clone(),
                token: token,
            });
        }

        for m in self.keys.iter() {
            let token = registry.get(&m.r#type)?;

            event.keys.push(EventInner {
                name: m.name.clone(),
                token: token,
            });
        }

        Ok(Token::Event(event))
    }
}

impl TokenConvertable for &RawLegacyStruct {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut structure = Struct::new(self.name.clone(), &registry)?;

        for field in self.members.iter() {
            let token = registry.get(&field.r#type)?;

            structure.fields.push(StructInner {
                name: field.name.clone(),
                token: token,
            });
        }

        Ok(Token::Struct(structure))
    }
}

impl TokenConvertable for &AbiFunction {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut function = Function::new(&self.name, self.state_mutability.clone().into());

        for input in self.inputs.iter() {
            let token = registry.get(&input.r#type)?;

            function.inputs.push(FuncInner {
                name: input.name.clone(),
                token: token,
            });
        }

        for output in self.outputs.iter() {
            let token = registry.get(&output.r#type)?;
            function.outputs.push(token);
        }

        Ok(Token::Function(function))
    }
}

impl TokenConvertable for &AbiInterface {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        let mut interface = Interface::new(&self.name)?;

        for item in self.items.iter() {
            let token = item.to_token(registry)?;
            // hmmmm, token name should be extracted from token
            // let token_ref = registry.set(token.type_path(), token);

            interface.functions.push(Rc::new(token));
        }

        Ok(Token::Interface(interface))
    }
}

impl TokenConvertable for AbiEntry {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
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
            // TODO: should be use for contract deployment (in the future)
            AbiEntry::Constructor(abi_constructor) => todo!(),
            AbiEntry::Impl(abi_impl) => todo!(),
            AbiEntry::Interface(abi_interface) => abi_interface.to_token(registry),
            AbiEntry::L1Handler(abi_function) => abi_function.to_token(registry),
        }
    }
}

impl Named for AbiEntry {
    fn get_name(&self) -> String {
        match &self {
            AbiEntry::Function(abi_function) => abi_function.name.clone(),
            AbiEntry::Event(AbiEvent::Typed(TypedAbiEvent::Enum(abi_event))) => {
                abi_event.name.clone()
            }
            AbiEntry::Event(AbiEvent::Typed(TypedAbiEvent::Struct(abi_event))) => {
                abi_event.name.clone()
            }
            AbiEntry::Event(AbiEvent::Untyped(abi_event)) => abi_event.name.clone(),
            AbiEntry::Struct(abi_struct) => abi_struct.name.clone(),
            AbiEntry::Enum(abi_enum) => abi_enum.name.clone(),
            // TODO: should be use for contract deployment (in the future)
            AbiEntry::Constructor(abi_constructor) => todo!(),
            AbiEntry::Impl(abi_impl) => todo!(),
            AbiEntry::Interface(abi_interface) => abi_interface.name.clone(),
            AbiEntry::L1Handler(abi_function) => abi_function.name.clone(),
        }
    }
}

impl TokenConvertable for LegacyEventAbiEntry {
    fn to_token(&self, registry: &mut TypeRegistry) -> Result<Token, Error> {
        todo!()
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
