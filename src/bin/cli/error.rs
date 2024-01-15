use cainome_parser::Error as CainomeError;
use starknet::providers::ProviderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Cainome(#[from] CainomeError),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("An error occurred: {0}")]
    Other(String),
}

pub type CainomeCliResult<T, E = Error> = Result<T, E>;
