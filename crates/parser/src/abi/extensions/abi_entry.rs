use std::{cell::RefCell, rc::Rc};

use starknet::core::types::{
    contract::{
        legacy::{RawLegacyEvent, RawLegacyStruct},
        AbiEntry, AbiEnum, AbiEvent, AbiEventEnum, AbiEventStruct, AbiFunction, AbiInterface,
        AbiStruct, EventFieldKind, TypedAbiEvent, UntypedAbiEvent,
    },
    LegacyEventAbiEntry,
};

use crate::{abi::registry::TypeRegistry, tokens::Interface};
use crate::{
    tokens::{Enum, EnumInner, Event, EventInner, FuncInner, Function, Struct, StructInner, Token},
    CainomeResult,
};

pub trait TokenConvertable: Sized {
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token>;
}

pub trait TryTokenConvertable: Sized {
    fn try_to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Option<Token>>;
}

impl<T> TryTokenConvertable for T
where
    T: TokenConvertable,
{
    fn try_to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Option<Token>> {
        return self.to_token(registry).map(|res| Some(res));
    }
}

pub trait Named {
    fn get_name(&self) -> String;
}

impl TokenConvertable for &AbiStruct {
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
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
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
        let mut interface = Interface::new(&self.name)?;

        for item in self.items.iter() {
            let AbiEntry::Function(func) = item else {
                // TODO: logging
                continue;
            };

            let token = func.to_token(registry)?;
            interface.functions.push(Rc::new(RefCell::new(token)));
        }

        Ok(Token::Interface(interface))
    }
}

impl TryTokenConvertable for AbiEntry {
    fn try_to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Option<Token>> {
        match self {
            AbiEntry::Function(abi_function) => abi_function.try_to_token(registry),
            AbiEntry::Event(AbiEvent::Typed(TypedAbiEvent::Enum(abi_event))) => {
                abi_event.try_to_token(registry)
            }
            AbiEntry::Event(AbiEvent::Typed(TypedAbiEvent::Struct(abi_event))) => {
                abi_event.try_to_token(registry)
            }
            AbiEntry::Event(AbiEvent::Untyped(abi_event)) => abi_event.try_to_token(registry),
            AbiEntry::Struct(abi_struct) => abi_struct.try_to_token(registry),
            AbiEntry::Enum(abi_enum) => abi_enum.try_to_token(registry),
            // TODO: should be use for contract deployment (in the future)
            AbiEntry::Constructor(abi_constructor) => todo!(),
            // TODO: Maybe rethink
            AbiEntry::Impl(abi_impl) => Ok(None),
            AbiEntry::Interface(abi_interface) => abi_interface.try_to_token(registry),
            AbiEntry::L1Handler(abi_function) => abi_function.try_to_token(registry),
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
            AbiEntry::Constructor(abi_constructor) => abi_constructor.name.clone(),
            AbiEntry::Impl(abi_impl) => abi_impl.name.clone(),
            AbiEntry::Interface(abi_interface) => abi_interface.name.clone(),
            AbiEntry::L1Handler(abi_function) => abi_function.name.clone(),
        }
    }
}

impl TokenConvertable for LegacyEventAbiEntry {
    fn to_token(&self, registry: &mut TypeRegistry) -> CainomeResult<Token> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::Container;
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
        let Token::Enum(enum_token) = &*result.enums[0].borrow() else {
            panic!("Should be enum");
        };

        assert_eq!(enum_token.type_path, "test::TestEnum");
        assert_eq!(enum_token.variants.len(), 3);

        // Check that variant without data has NotUsed kind
        assert_eq!(enum_token.variants[0].name, "VariantWithoutData");
        let Token::Basic(f1t) = &*enum_token.variants[0].token.borrow() else {
            panic!("First field token should be basic");
        };
        assert_eq!(f1t.type_path, "()");

        // Check that variant with felt252 has Data kind
        assert_eq!(enum_token.variants[1].name, "VariantWithFelt252");
        let Token::Basic(f2t) = &*enum_token.variants[1].token.borrow() else {
            panic!("Second field token should be basic");
        };
        assert_eq!(f2t.type_path, "core::felt252");

        // Check that variant with tuple has Data kind
        assert_eq!(enum_token.variants[2].name, "VariantWithTuple");
        let Token::Tuple(f3t) = &*enum_token.variants[2].token.borrow() else {
            panic!("Third field token should be basic");
        };
        assert_eq!(f3t.type_path, "(core::felt252, core::integer::u32)");
    }
}
