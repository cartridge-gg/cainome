use super::CairoSerde;

use starknet::core::types::FieldElement;

/// Cairo types result.
pub type Result<T> = core::result::Result<T, Error>;

/// A cairo type error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Invalid type found {0:?}.")]
    InvalidTypeString(String),
    #[error("Error during serialization {0:?}.")]
    Serialize(String),
    #[error("Error during deserialization {0:?}.")]
    Deserialize(String),
}

impl CairoSerde for Error {
    type RustType = Self;

    fn cairo_serialize(_rust: &Self::RustType) -> Vec<FieldElement> {
        vec![]
    }

    fn cairo_deserialize(_felts: &[FieldElement], _offset: usize) -> Result<Self::RustType> {
        Ok(Error::Deserialize(
            "Error cairotype deserialized?".to_string(),
        ))
    }
}
