//! # Execution version of Starknet transactions.

/// The version of transaction to be executed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExecutionVersion {
    /// Execute the transaction using the `execute_v3` method, where fees are payable in WEI or FRI.
    #[default]
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
            "Invalid execution version '{}'. Supported values are 'v3' or 'V3'.",
            self.invalid_value
        )
    }
}

impl std::error::Error for ParseExecutionVersionError {}

impl std::str::FromStr for ExecutionVersion {
    type Err = ParseExecutionVersionError;

    fn from_str(input: &str) -> Result<ExecutionVersion, Self::Err> {
        match input {
            "v3" | "V3" => Ok(ExecutionVersion::V3),
            _ => Err(ParseExecutionVersionError {
                invalid_value: input.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExecutionVersion;
    use std::str::FromStr;

    #[test]
    fn parses_v3() {
        assert_eq!(
            ExecutionVersion::from_str("v3").unwrap(),
            ExecutionVersion::V3
        );
        assert_eq!(
            ExecutionVersion::from_str("V3").unwrap(),
            ExecutionVersion::V3
        );
    }

    #[test]
    fn rejects_removed_v1() {
        assert!(ExecutionVersion::from_str("v1").is_err());
        assert!(ExecutionVersion::from_str("V1").is_err());
    }
}
