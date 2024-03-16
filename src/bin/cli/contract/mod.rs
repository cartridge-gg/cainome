use cainome_parser::{AbiParser, TokenizedAbi};
use camino::Utf8PathBuf;
use std::collections::HashMap;
use std::fs;
use url::Url;

// use starknet::core::types::contract::*;

use starknet::{
    core::types::{BlockId, BlockTag, ContractClass, FieldElement},
    providers::{jsonrpc::HttpTransport, AnyProvider, JsonRpcClient, Provider},
};

use crate::error::{CainomeCliResult, Error};

#[derive(Debug)]
pub enum ContractOrigin {
    /// Contract's ABI was loaded from a local Sierra class file
    /// with the given file name.
    SierraClassFile(String),
    /// Contract's ABI was fetched from the given address.
    FetchedFromChain(FieldElement),
}

#[derive(Debug)]
pub struct ContractData {
    /// Contract's name.
    pub name: String,
    /// Contract's origin.
    pub origin: ContractOrigin,
    /// Tokens parsed from the ABI.
    pub tokens: TokenizedAbi,
}

pub struct ContractParser {}

impl ContractParser {
    pub fn from_artifacts_path(
        path: Utf8PathBuf,
        sierra_ext: &str,
    ) -> CainomeCliResult<Vec<ContractData>> {
        let mut contracts = vec![];

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if !file_name.ends_with(sierra_ext) {
                        continue;
                    }

                    let file_content = fs::read_to_string(&path)?;

                    // TODO: check how the aliases can be passed to the CLI....!
                    // It's a simple HashMap<String, String>, flat file with two columns
                    // may be ok?
                    let aliases = HashMap::new();

                    match AbiParser::tokens_from_abi_string(&file_content, &aliases) {
                        Ok(tokens) => {
                            let contract_name = file_name.trim_end_matches(sierra_ext);

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
            }
        }

        Ok(contracts)
    }

    pub async fn from_chain(
        name: &str,
        address: FieldElement,
        rpc_url: Url,
    ) -> CainomeCliResult<ContractData> {
        let provider = AnyProvider::JsonRpcHttp(JsonRpcClient::new(HttpTransport::new(rpc_url)));

        let class = provider
            .get_class_at(BlockId::Tag(BlockTag::Latest), address)
            .await?;

        // TODO: check how the aliases can be passed to the CLI....!
        // It's a simple HashMap<String, String>, flat file with two columns
        // may be ok?
        let aliases = HashMap::new();

        match class {
            ContractClass::Sierra(sierra) => {
                match AbiParser::tokens_from_abi_string(&sierra.abi, &aliases) {
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
