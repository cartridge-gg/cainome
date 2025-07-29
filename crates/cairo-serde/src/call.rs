//! This file must be in the proc_macro2 crate that must be reworked.
use starknet::core::types::{BlockId, BlockTag, FunctionCall};
use std::marker::PhantomData;

use crate::{CairoSerde, Error, Result as CairoResult};

#[derive(Debug)]
pub struct FCall<'p, P, T> {
    pub call_raw: FunctionCall,
    pub block_id: BlockId,
    provider: &'p P,
    rust_type: PhantomData<T>,
}

impl<'p, P, T> FCall<'p, P, T>
where
    P: starknet::providers::Provider + Sync,
    T: CairoSerde<RustType = T>,
{
    pub fn new(call_raw: FunctionCall, provider: &'p P) -> Self {
        Self {
            call_raw,
            block_id: BlockId::Tag(BlockTag::PreConfirmed),
            provider,
            rust_type: PhantomData,
        }
    }

    pub fn provider(self) -> &'p P {
        self.provider
    }

    pub fn block_id(self, block_id: BlockId) -> Self {
        Self { block_id, ..self }
    }

    pub async fn call(self) -> CairoResult<T> {
        let r = self
            .provider
            .call(self.call_raw, self.block_id)
            .await
            .map_err(Error::Provider)?;

        T::cairo_deserialize(&r, 0)
    }

    pub async fn raw_call(self) -> CairoResult<Vec<starknet::core::types::Felt>> {
        self.provider
            .call(self.call_raw, self.block_id)
            .await
            .map_err(Error::Provider)
    }
}
