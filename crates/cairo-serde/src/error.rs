use super::CairoSerde;

use starknet::{core::types::Felt, providers::ProviderError};

/// Cairo types result.
pub type Result<T> = core::result::Result<T, Error>;

/// A cairo type error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid type found {0:?}.")]
    InvalidTypeString(String),
    #[error("Error during serialization {0:?}.")]
    Serialize(String),
    #[error("Error during deserialization {0:?}.")]
    Deserialize(String),
    #[error("Provider errror {0:?}.")]
    Provider(#[from] ProviderError),
    #[error("Bytes31 out of range.")]
    Bytes31OutOfRange,
    #[error("NonZero that is zero")]
    ZeroedNonZero,
}

impl CairoSerde for Error {
    type RustType = Self;

    fn cairo_serialize(_rust: &Self::RustType) -> Vec<Felt> {
        vec![]
    }

    fn cairo_deserialize(_felts: &[Felt], _offset: usize) -> Result<Self::RustType> {
        Ok(Error::Deserialize(
            "Error cairotype deserialized?".to_string(),
        ))
    }
}
