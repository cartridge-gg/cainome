use cainome_parser::{AbiParser, TokenizedAbi};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use url::Url;

use starknet::{
    core::types::{BlockId, BlockTag, ContractClass, Felt},
    providers::{jsonrpc::HttpTransport, AnyProvider, JsonRpcClient, Provider},
};

use crate::error::{CainomeCliResult, Error};

#[derive(Debug)]
#[allow(dead_code)]
pub enum ContractOrigin {
    /// Contract's ABI was loaded from a local Sierra class file
    /// with the given file name.
    SierraClassFile(String),
    /// Contract's ABI was fetched from the given address.
    FetchedFromChain(Felt),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ContractData {
    /// Contract's name.
    pub name: String,
    /// Contract's origin.
    pub origin: ContractOrigin,
    /// Tokens parsed from the ABI.
    pub tokens: TokenizedAbi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractParserConfig {
    /// The file extension that should be considered as a Sierra file.
    pub sierra_extension: String,
    /// The type aliases to be provided to the Cainome parser.
    pub type_aliases: HashMap<String, String>,
    /// The contract aliases to be provided to the Cainome parser.
    pub contract_aliases: HashMap<String, String>,
}

impl ContractParserConfig {
    pub fn from_json(path: &Utf8PathBuf) -> CainomeCliResult<Self> {
        Ok(serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(path)?,
        ))?)
    }
}

impl Default for ContractParserConfig {
    fn default() -> Self {
        Self {
            sierra_extension: ".contract_class.json".to_string(),
            type_aliases: HashMap::default(),
            contract_aliases: HashMap::default(),
        }
    }
}

pub struct ContractParser {}

impl ContractParser {
    pub fn from_artifacts_path(
        path: Utf8PathBuf,
        config: &ContractParserConfig,
    ) -> CainomeCliResult<Vec<ContractData>> {
        let mut contracts = vec![];
        let path_str = path.as_str();

        // Check if the path contains glob patterns
        let file_paths = if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            // Handle glob pattern
            tracing::trace!("Processing glob pattern: {}", path_str);
            let glob_paths = glob::glob(path_str)
                .map_err(|e| Error::Other(format!("Invalid glob pattern '{}': {}", path_str, e)))?;
            
            let mut paths = Vec::new();
            for glob_result in glob_paths {
                match glob_result {
                    Ok(path) => paths.push(path),
                    Err(e) => tracing::warn!("Error processing glob entry: {}", e),
                }
            }
            paths
        } else {
            // Handle directory path (existing behavior)
            let path_obj = Path::new(path_str);
            if path_obj.is_dir() {
                tracing::trace!("Processing directory: {}", path_str);
                let mut paths = Vec::new();
                for entry in fs::read_dir(path_obj)? {
                    let entry = entry?;
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        paths.push(entry_path);
                    }
                }
                paths
            } else if path_obj.is_file() {
                // Handle single file path
                tracing::trace!("Processing single file: {}", path_str);
                vec![path_obj.to_path_buf()]
            } else {
                return Err(Error::Other(format!("Path '{}' does not exist or is not accessible", path_str)));
            }
        };

        // Process all collected file paths
        for file_path in file_paths {
            if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                if !file_name.ends_with(&config.sierra_extension) {
                    continue;
                }

                match fs::read_to_string(&file_path) {
                    Ok(file_content) => {
                        match AbiParser::tokens_from_abi_string(&file_content, &config.type_aliases) {
                            Ok(tokens) => {
                                let contract_name = {
                                    let n = file_name.trim_end_matches(&config.sierra_extension);
                                    if let Some(alias) = config.contract_aliases.get(n) {
                                        tracing::trace!(
                                            "Aliasing {file_name} contract name with {alias}"
                                        );
                                        alias
                                    } else {
                                        n
                                    }
                                };

                                tracing::trace!(
                                    "Adding {contract_name} ({file_name}) to the list of contracts"
                                );
                                contracts.push(ContractData {
                                    name: contract_name.to_string(),
                                    origin: ContractOrigin::SierraClassFile(file_name.to_string()),
                                    tokens,
                                });
                            }
                            Err(e) => {
                                tracing::warn!("Sierra file {file_name} could not be parsed {e:?}")
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Could not read file {}: {}", file_path.display(), e);
                    }
                }
            }
        }

        Ok(contracts)
    }

    pub async fn from_chain(
        name: &str,
        address: Felt,
        rpc_url: Url,
        type_aliases: &HashMap<String, String>,
    ) -> CainomeCliResult<ContractData> {
        let provider = AnyProvider::JsonRpcHttp(JsonRpcClient::new(HttpTransport::new(rpc_url)));

        let class = provider
            .get_class_at(BlockId::Tag(BlockTag::Latest), address)
            .await?;

        match class {
            ContractClass::Sierra(sierra) => {
                match AbiParser::tokens_from_abi_string(&sierra.abi, type_aliases) {
                    Ok(tokens) => Ok(ContractData {
                        name: name.to_string(),
                        origin: ContractOrigin::FetchedFromChain(address),
                        tokens,
                    }),
                    Err(e) => Err(Error::Other(format!(
                        "Error parsing ABI from address {:#x}: {:?}",
                        address, e
                    ))),
                }
            }
            ContractClass::Legacy(_) => Err(Error::Other(
                "Legacy class is not supported yet".to_string(),
            )),
        }
    }
}
