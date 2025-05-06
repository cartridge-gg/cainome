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
    #[error("Invalid option type path: {0}")]
    InvalidOptionTypePath(String),
    #[error("Invalid result type path: {0}")]
    InvalidResultTypePath(String),
    #[error("Invalid non-zero type path: {0}")]
    InvalidNonZeroTypePath(String),
}

pub type CainomeResult<T, E = Error> = Result<T, E>;
