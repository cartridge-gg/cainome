//! Defines the arguments of the `abigen` macro.
//!
//! `ContractAbi` is expected to be the argument
//! passed to the macro. We should then parse the
//! token stream to ensure the arguments are correct.
//!
//! At this moment, the macro supports one fashion:
//!
//! Loading from a file with only the ABI array.
//! abigen!(ContractName, "path/to/abi.json"
//!
//! TODO: support the full artifact JSON to be able to
//! deploy contracts from abigen.
use quote::ToTokens;
use starknet::core::types::contract::{AbiEntry, SierraClass};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::str::FromStr;
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream, Result},
    Ident, LitStr, Token, Type,
};

use crate::spanned::Spanned;
use cainome_rs::ExecutionVersion;

const CARGO_MANIFEST_DIR: &str = "$CARGO_MANIFEST_DIR/";

#[derive(Clone, Debug)]
pub(crate) struct ContractAbi {
    pub name: Ident,
    pub abi: Vec<AbiEntry>,
    pub output_path: Option<String>,
    pub type_aliases: HashMap<String, String>,
    pub execution_version: ExecutionVersion,
    pub derives: Vec<String>,
}

impl Parse for ContractAbi {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;

        // ABI path or content.

        // Path rooted to the Cargo.toml location if it's a file.
        let abi_or_path = input.parse::<LitStr>()?;

        #[allow(clippy::collapsible_else_if)]
        let abi = if abi_or_path.value().ends_with(".json") {
            let json_path = if abi_or_path.value().starts_with(CARGO_MANIFEST_DIR) {
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                let new_dir = Path::new(manifest_dir)
                    .join(abi_or_path.value().trim_start_matches(CARGO_MANIFEST_DIR))
                    .to_string_lossy()
                    .to_string();

                LitStr::new(&new_dir, proc_macro2::Span::call_site())
            } else {
                abi_or_path
            };

            // To prepare the declare and deploy features, we also
            // accept a full Sierra artifact for the ABI.
            // To support declare and deploy, the full class must be stored.
            if let Ok(sierra) =
                serde_json::from_reader::<_, SierraClass>(open_json_file(&json_path.value())?)
            {
                sierra.abi
            } else {
                serde_json::from_reader::<_, Vec<AbiEntry>>(open_json_file(&json_path.value())?)
                    .map_err(|e| {
                        syn::Error::new(json_path.span(), format!("JSON parse error: {}", e))
                    })?
            }
        } else {
            if let Ok(sierra) = serde_json::from_str::<SierraClass>(&abi_or_path.value()) {
                sierra.abi
            } else {
                serde_json::from_str::<Vec<AbiEntry>>(&abi_or_path.value()).map_err(|e| {
                    syn::Error::new(abi_or_path.span(), format!("JSON parse error: {}", e))
                })?
            }
        };

        let mut output_path: Option<String> = None;
        let mut execution_version = ExecutionVersion::V1;
        let mut type_aliases = HashMap::new();
        let mut derives = Vec::new();

        loop {
            if input.parse::<Token![,]>().is_err() {
                break;
            }

            let name = match Ident::parse_any(input) {
                Ok(n) => n,
                Err(_) => break,
            };

            match name.to_string().as_str() {
                "type_aliases" => {
                    let content;
                    braced!(content in input);
                    let parsed =
                        content.parse_terminated(Spanned::<TypeAlias>::parse, Token![;])?;

                    let mut abi_types = HashSet::new();
                    let mut aliases = HashSet::new();

                    for type_alias in parsed {
                        if !abi_types.insert(type_alias.abi.clone()) {
                            panic!("{} duplicate abi type", type_alias.abi)
                        }
                        if !aliases.insert(type_alias.alias.clone()) {
                            panic!("{} duplicate alias name", type_alias.alias)
                        }

                        let ta = type_alias.into_inner();
                        type_aliases.insert(ta.abi, ta.alias);
                    }
                }
                "output_path" => {
                    let content;
                    parenthesized!(content in input);
                    output_path = Some(content.parse::<LitStr>()?.value());
                }
                "execution_version" => {
                    let content;
                    parenthesized!(content in input);
                    let ev = content.parse::<LitStr>()?.value();
                    execution_version = ExecutionVersion::from_str(&ev).map_err(|e| {
                        syn::Error::new(content.span(), format!("Invalid execution version: {}", e))
                    })?;
                }
                "derives" => {
                    let content;
                    parenthesized!(content in input);
                    let parsed = content.parse_terminated(Spanned::<Type>::parse, Token![,])?;

                    for derive in parsed {
                        derives.push(derive.to_token_stream().to_string());
                    }
                }
                _ => panic!("unexpected named parameter `{}`", name),
            }
        }

        Ok(ContractAbi {
            name,
            abi,
            output_path,
            type_aliases,
            execution_version,
            derives,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypeAlias {
    abi: String,
    alias: String,
}

impl Parse for TypeAlias {
    fn parse(input: ParseStream) -> Result<Self> {
        let abi = input
            .parse::<Type>()?
            .into_token_stream()
            .to_string()
            .replace(' ', "");

        input.parse::<Token![as]>()?;

        let alias = input.parse::<Ident>()?.to_string();

        Ok(TypeAlias { abi, alias })
    }
}

fn open_json_file(file_path: &str) -> Result<File> {
    File::open(file_path).map_err(|e| {
        syn::Error::new(
            str_to_litstr(file_path).span(),
            format!("JSON open file {} error: {}", file_path, e),
        )
    })
}

pub fn str_to_litstr(str_in: &str) -> LitStr {
    LitStr::new(str_in, proc_macro::Span::call_site().into())
}
