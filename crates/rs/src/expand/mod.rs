use std::collections::{BTreeSet, HashMap, HashSet};

use cainome_parser::ParserContext;
use proc_macro2::TokenStream;

use crate::ExecutionVersion;
mod module;
pub(crate) use module::Module;

#[cfg(test)]
mod for_tests;

#[cfg(test)]
mod module_tests;

pub(crate) mod contract;

#[cfg(test)]
mod contract_tests;

pub(crate) mod enumeration;

#[cfg(test)]
mod enumeration_tests;

pub(crate) mod event;

#[cfg(test)]
mod event_tests;

pub(crate) mod structure;

#[cfg(test)]
mod structure_tests;

mod types;
pub mod utils;

pub const ROOT_MODULE_NAME: &str = "";

#[derive(Clone)]
pub struct ExpansionResult {
    pub name: String,
    pub path: Vec<String>,
    pub content: HashMap<String, TokenStream>,
    pub imports: HashSet<String>,
}

impl ExpansionResult {
    pub fn new(full_path: &str) -> Self {
        let segments = full_path
            .split("::")
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        let res = if let Option::Some((name, path)) = segments.split_last() {
            Self {
                name: name.to_string(),
                path: path.to_vec(),
                content: HashMap::new(),
                imports: HashSet::new(),
            }
        } else {
            Self {
                name: full_path.to_string(),
                path: vec![],
                content: HashMap::new(),
                imports: HashSet::new(),
            }
        };

        res
    }

    pub fn with_item(mut self, item_name: &str, item: TokenStream) -> Self {
        self.content.insert(item_name.to_string(), item);
        self
    }

    pub fn with_imports<I, S>(mut self, imports: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for import in imports {
            self.imports.insert(import.as_ref().to_string());
        }
        self
    }
}

#[derive(Clone)]
pub struct ExpansionContext {
    pub contract_name: String,
    pub derives: BTreeSet<String>,
    pub contract_derives: BTreeSet<String>,
    pub execution_version: ExecutionVersion,
    pub substitutions: HashMap<String, String>,
    pub type_skips: Vec<String>,
    pub aliases: HashMap<String, String>,
    // TODO: syn::Type?
    pub root_module_path: String,
}

impl Default for ExpansionContext {
    fn default() -> Self {
        Self::new("DefaultContract")
    }
}

impl ExpansionContext {
    pub fn new(contract_name: &str) -> Self {
        let ccsp = utils::cainome_cairo_serde_path();
        let snrs_types = utils::starknet_rs_types_path();

        let builtin_substitutions = HashMap::from([
            ("core::bool", "bool".to_string()),
            ("core::integer::u8", "u8".to_string()),
            ("core::integer::u16", "u16".to_string()),
            ("core::integer::u32", "u32".to_string()),
            ("core::integer::u64", "u64".to_string()),
            ("core::integer::u128", "u128".to_string()),
            ("core::integer::usize", "usize".to_string()),
            ("core::integer::i8", "i8".to_string()),
            ("core::integer::i16", "i16".to_string()),
            ("core::integer::i32", "i32".to_string()),
            ("core::integer::i64", "i64".to_string()),
            ("core::integer::i128", "i128".to_string()),
            (
                "core::starknet::eth_address::EthAddress",
                format!("{ccsp}::EthAddress"),
            ),
            (
                "core::starknet::class_hash::ClassHash",
                format!("{ccsp}::ClassHash"),
            ),
            (
                "core::starknet::contract_address::ContractAddress",
                format!("{ccsp}::ContractAddress"),
            ),
            ("core::byte_array::ByteArray", format!("{ccsp}::ByteArray")),
            ("core::zeroable::NonZero", format!("{ccsp}::NonZero")),
            ("core::integer::u256", format!("{ccsp}::U256")),
            ("core::integer::BoundedInt", format!("{snrs_types}::Felt")),
            ("felt", format!("{snrs_types}::Felt")),
            ("core::felt252", format!("{snrs_types}::Felt")),
            ("core::bytes_31::bytes31", format!("{ccsp}::Bytes31")),
        ])
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        Self {
            derives: BTreeSet::new(),
            contract_derives: BTreeSet::new(),
            contract_name: contract_name.to_string(),
            execution_version: crate::ExecutionVersion::V3,
            root_module_path: "crate".to_string(),
            substitutions: builtin_substitutions,
            type_skips: vec![],
            aliases: HashMap::new(),
        }
    }

    pub fn with_execution(mut self, execution_version: ExecutionVersion) -> Self {
        self.execution_version = execution_version;
        self
    }

    pub fn with_v1_execution(self) -> Self {
        self.with_execution(ExecutionVersion::V1)
    }

    pub fn with_derives<I, S>(mut self, derives: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for derive in derives {
            if derive.as_ref() == "Serde" {
                self.derives.insert("serde::Serialize".to_string());
                self.derives.insert("serde::Deserialize".to_string());
                continue;
            }

            self.derives.insert(derive.as_ref().to_string());
        }
        self
    }

    pub fn with_contract_derives<I, S>(mut self, derives: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for derive in derives {
            self.contract_derives.insert(derive.as_ref().to_string());
        }
        self
    }

    pub fn with_substitutions(mut self, substitutions: HashMap<String, String>) -> Self {
        for (k, v) in substitutions {
            self.substitutions.insert(k, v);
        }
        self
    }

    pub fn with_type_skips<I, S>(mut self, type_skips: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for type_skip in type_skips {
            self.type_skips.push(type_skip.as_ref().to_string());
        }
        self
    }

    pub fn with_aliases(mut self, aliases: HashMap<String, String>) -> Self {
        for (k, v) in aliases {
            self.aliases.insert(k, v);
        }
        self
    }

    pub fn apply_alias(&self, type_path_no_generic: &str) -> String {
        if let Some(alias) = self.aliases.get(type_path_no_generic) {
            alias.to_string()
        } else {
            type_path_no_generic.to_string()
        }
    }
}

impl From<&ExpansionContext> for ParserContext {
    fn from(value: &ExpansionContext) -> Self {
        ParserContext::new()
            .with_substitutions(
                value
                    .substitutions
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect(),
            )
            .with_type_skips(value.type_skips.iter().map(|s| s.as_str()).collect())
    }
}

pub trait Expandable {
    fn expand(&self, ctx: &ExpansionContext) -> Vec<ExpansionResult>;
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_expand_empty() {}
}
