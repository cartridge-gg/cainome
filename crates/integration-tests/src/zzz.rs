pub mod cainome {
    pub use cainome_cairo_serde as cairo_serde;
}

pub mod my {
    #[derive(Serde, Clone)]
    pub enum Type {
        f1(cainome::cairo_serde::NonZero<starknet::core::types::Felt>),
        f2(Option<starknet::core::types::Felt>),
        f3(Vec<starknet::core::types::Felt>),
        f4(
            (
                starknet::core::types::Felt,
                Option<starknet::core::types::Felt>,
            ),
        ),
    }
    impl cainome::cairo_serde::CairoSerde for Type {
        type RustType = Self;
        const SERIALIZED_SIZE: std::option::Option<usize> = std::option::Option::None;
        #[inline]
        fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
            match __rust { Type :: f1 (val) => cainome :: cairo_serde :: NonZero :: < starknet :: core :: types :: Felt > :: cairo_serialized_size (val) + 1 , Type :: f2 (val) => Option :: < starknet :: core :: types :: Felt > :: cairo_serialized_size (val) + 1 , Type :: f3 (val) => Vec :: < starknet :: core :: types :: Felt > :: cairo_serialized_size (val) + 1 , Type :: f4 (val) => < (starknet :: core :: types :: Felt , Option :: < starknet :: core :: types :: Felt >) > :: cairo_serialized_size (val) + 1 , _ => 0 }
        }
        fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
            match __rust {
                Type::f1(val) => {
                    let mut temp = vec![];
                    temp.extend(usize::cairo_serialize(&0usize));
                    temp . extend (cainome :: cairo_serde :: NonZero :: < starknet :: core :: types :: Felt > :: cairo_serialize (val)) ;
                    temp
                }
                Type::f2(val) => {
                    let mut temp = vec![];
                    temp.extend(usize::cairo_serialize(&1usize));
                    temp.extend(Option::<starknet::core::types::Felt>::cairo_serialize(val));
                    temp
                }
                Type::f3(val) => {
                    let mut temp = vec![];
                    temp.extend(usize::cairo_serialize(&2usize));
                    temp.extend(Vec::<starknet::core::types::Felt>::cairo_serialize(val));
                    temp
                }
                Type::f4(val) => {
                    let mut temp = vec![];
                    temp.extend(usize::cairo_serialize(&3usize));
                    temp.extend(<(
                        starknet::core::types::Felt,
                        Option<starknet::core::types::Felt>,
                    )>::cairo_serialize(val));
                    temp
                }
                _ => vec![],
            }
        }
        fn cairo_deserialize(
            __felts: &[starknet::core::types::Felt],
            __offset: usize,
        ) -> cainome::cairo_serde::Result<Self::RustType> {
            let __f = __felts[__offset];
            let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());
            match __index as usize {
                0usize => Ok(Type::f1(cainome::cairo_serde::NonZero::<
                    starknet::core::types::Felt,
                >::cairo_deserialize(
                    __felts, __offset + 1
                )?)),
                1usize => Ok(Type::f2(
                    Option::<starknet::core::types::Felt>::cairo_deserialize(
                        __felts,
                        __offset + 1,
                    )?,
                )),
                2usize => Ok(Type::f3(
                    Vec::<starknet::core::types::Felt>::cairo_deserialize(__felts, __offset + 1)?,
                )),
                3usize => Ok(Type::f4(<(
                    starknet::core::types::Felt,
                    Option<starknet::core::types::Felt>,
                )>::cairo_deserialize(
                    __felts, __offset + 1
                )?)),
                _ => {
                    return Err(cainome::cairo_serde::Error::Deserialize(format!(
                        "Index not handle for enum {}",
                        "Type"
                    )))
                }
            }
        }
    }
}
