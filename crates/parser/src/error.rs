use syn::Error as SynError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Syn(#[from] SynError),
    #[error("Token initialization error: {0}")]
    TokenInitFailed(String),
    #[error("Conversion error: {0}")]
    ConversionFailed(String),
    #[error("Parser error: {0}")]
    ParsingFailed(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

pub type CainomeResult<T, E = Error> = Result<T, E>;
