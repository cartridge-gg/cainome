use cainome_cairo_serde_derive::CairoSerde as CairoSerdeMacro;

#[derive(Clone, Debug, PartialEq, CairoSerdeMacro)]
pub struct E1<T> {
    pub key: T,
    pub value: Vec<starknet::core::types::Felt>,
}

#[derive(Clone, Debug, PartialEq, CairoSerdeMacro)]
pub enum E2<T> {
    Variant1 { data: T },
    Variant2(starknet::core::types::Felt, T),
}
