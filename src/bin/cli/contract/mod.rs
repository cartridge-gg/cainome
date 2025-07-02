use cainome_parser::{AbiParser, TokenizedAbi};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
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
    /// Optional list of specific contract files to include from the artifacts path.
    /// If not specified, all files with the sierra_extension will be included.
    /// File names should include the extension (e.g., "my_contract.contract_class.json").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub included_contracts: Option<Vec<String>>,
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
            included_contracts: None,
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

        // Collect files to process based on configuration
        let files_to_process: Vec<String> =
            if let Some(included_contracts) = &config.included_contracts {
                // Use explicitly specified contracts
                included_contracts.clone()
            } else {
                // Discover all files with the sierra extension
                fs::read_dir(&path)?
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let file_path = entry.path();

                        if file_path.is_file() {
                            let file_name = file_path.file_name()?.to_str()?;
                            if file_name.ends_with(&config.sierra_extension) {
                                Some(file_name.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            };

        // Process each file
        for file_name in files_to_process {
            let file_path = path.join(&file_name);

            // Validate file exists (important for explicitly specified files)
            if !file_path.exists() {
                tracing::warn!("Contract file '{}' not found in artifacts path", file_name);
                continue;
            }

            // Validate file extension
            if !file_name.ends_with(&config.sierra_extension) {
                tracing::warn!(
                    "Contract file '{}' does not have the expected extension '{}'",
                    file_name,
                    config.sierra_extension
                );
                continue;
            }

            // Read and parse the file
            let file_content = fs::read_to_string(&file_path)?;

            match AbiParser::tokens_from_abi_string(&file_content, &config.type_aliases) {
                Ok(tokens) => {
                    let contract_name = {
                        let n = file_name.trim_end_matches(&config.sierra_extension);
                        if let Some(alias) = config.contract_aliases.get(n) {
                            tracing::trace!("Aliasing {file_name} contract name with {alias}");
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
