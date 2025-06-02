//! # Execution version of Starknet transactions.

/// The version of transaction to be executed.
#[derive(Debug, Clone, Copy, Default)]
pub enum ExecutionVersion {
    /// Execute the transaction using the `execute_v1` method, where fees are only payable in WEI.
    #[default]
    V1,
    /// Execute the transaction using the `execute_v3` method, where fees are payable in WEI or FRI.
    V3,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseExecutionVersionError {
    invalid_value: String,
}

impl std::fmt::Display for ParseExecutionVersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid execution version '{}'. Supported values are 'v1', 'V1', 'v3', or 'V3'.",
            self.invalid_value
        )
    }
}

impl std::error::Error for ParseExecutionVersionError {}

impl std::str::FromStr for ExecutionVersion {
    type Err = ParseExecutionVersionError;

    fn from_str(input: &str) -> Result<ExecutionVersion, Self::Err> {
        match input {
            "v1" | "V1" => Ok(ExecutionVersion::V1),
            "v3" | "V3" => Ok(ExecutionVersion::V3),
            _ => Err(ParseExecutionVersionError {
                invalid_value: input.to_string(),
            }),
        }
    }
}
