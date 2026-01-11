use std::collections::{BTreeSet, HashMap, HashSet};

use cainome_parser::ParserContext;
use proc_macro2::TokenStream;

use crate::ExecutionVersion;
mod module;
pub(crate) use module::Module;

#[cfg(test)]
pub mod for_tests;

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

        // println!("Created ExpansionResult for: {}", full_path);

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
    // TODO: expose properties through methods
    pub contract_name: String,
    pub derives: BTreeSet<String>,
    pub contract_derives: BTreeSet<String>,
    pub execution_version: ExecutionVersion,
    pub substitutions: HashMap<String, String>,
    pub type_skips: Vec<String>,
    pub aliases: HashMap<String, String>,
    pub contract_source: String,
    pub add_declaration: bool,
    pub add_deployment: bool,

    // TODO: syn::Type?
    pub root_module_path: String,
    pub cainome_serde_path: String,
    pub is_legacy: bool,

    // TODO: proxy configuration
    pub sierra_max_bytecode_size: usize,
    pub sierra_add_pythonic_hints: bool,
    pub deployer_generate_salt: bool,
    pub deployer_is_unique: bool,
}

impl ExpansionContext {
    pub fn apply_alias(&self, type_path_no_generic: &str) -> String {
        if let Some(alias) = self.aliases.get(type_path_no_generic) {
            alias.to_string()
        } else {
            type_path_no_generic.to_string()
        }
    }
}

pub struct ExpansionContextFactory {
    contract_name: String,
    derives: BTreeSet<String>,
    contract_derives: BTreeSet<String>,
    execution_version: ExecutionVersion,
    substitutions: HashMap<String, String>,
    type_skips: Vec<String>,
    aliases: HashMap<String, String>,
    contract_source: String,
    add_declaration: bool,
    add_deployment: bool,
    // TODO: syn::Type?
    root_module_path: String,
    cainome_serde_path: String,
    is_legacy: bool,
}

impl ExpansionContextFactory {
    pub fn new<S>(contract_source: S) -> Self
    where
        S: AsRef<str>,
    {
        Self {
            derives: BTreeSet::new(),
            contract_derives: BTreeSet::new(),
            contract_name: "Contract".to_string(),
            execution_version: crate::ExecutionVersion::V3,
            root_module_path: "self".to_string(),
            substitutions: HashMap::new(),
            type_skips: vec![],
            aliases: HashMap::new(),
            contract_source: contract_source.as_ref().to_string(),
            cainome_serde_path: "cainome::cairo_serde".to_string(),
            is_legacy: false,
            add_declaration: true,
            add_deployment: true,
        }
    }

    pub fn with_execution(mut self, execution_version: ExecutionVersion) -> Self {
        self.execution_version = execution_version;
        self
    }

    pub fn with_is_legacy(mut self, is_legacy: bool) -> Self {
        self.is_legacy = is_legacy;
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

    pub fn with_contract_name<S>(mut self, name: S) -> Self
    where
        S: AsRef<str>,
    {
        self.contract_name = name.as_ref().to_string();
        self
    }

    pub fn with_cainome_serde_path<S>(mut self, path: S) -> Self
    where
        S: AsRef<str>,
    {
        self.cainome_serde_path = path.as_ref().to_string();
        self
    }

    pub fn with_root_module_path<S>(mut self, path: S) -> Self
    where
        S: AsRef<str>,
    {
        self.root_module_path = path.as_ref().to_string();
        self
    }

    pub fn with_add_declaration(mut self, declare: bool) -> Self {
        self.add_declaration = declare;
        self
    }

    pub fn with_add_deployment(mut self, deploy: bool) -> Self {
        self.add_deployment = deploy;
        self
    }

    pub fn build(self) -> ExpansionContext {
        let cainome_serde_path = self.cainome_serde_path;
        let snrs_types = utils::starknet_rs_types_path();

        let mut builtin_substitutions: HashMap<String, String> = HashMap::from([
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
                format!("{cainome_serde_path}::EthAddress"),
            ),
            (
                "core::starknet::class_hash::ClassHash",
                format!("{cainome_serde_path}::ClassHash"),
            ),
            (
                "core::starknet::contract_address::ContractAddress",
                format!("{cainome_serde_path}::ContractAddress"),
            ),
            (
                "core::byte_array::ByteArray",
                format!("{cainome_serde_path}::ByteArray"),
            ),
            (
                "core::zeroable::NonZero",
                format!("{cainome_serde_path}::NonZero"),
            ),
            ("Uint256", format!("{cainome_serde_path}::U256")),
            ("core::integer::u256", format!("{cainome_serde_path}::U256")),
            ("core::integer::BoundedInt", format!("{snrs_types}::Felt")),
            ("felt", format!("{snrs_types}::Felt")),
            ("core::felt252", format!("{snrs_types}::Felt")),
            (
                "core::bytes_31::bytes31",
                format!("{cainome_serde_path}::Bytes31"),
            ),
        ])
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        builtin_substitutions.extend(self.substitutions);

        ExpansionContext {
            contract_name: self.contract_name,
            derives: self.derives,
            contract_derives: self.contract_derives,
            execution_version: self.execution_version,
            substitutions: builtin_substitutions,
            type_skips: self.type_skips,
            aliases: self.aliases,
            contract_source: self.contract_source,
            root_module_path: self.root_module_path,
            cainome_serde_path,
            is_legacy: self.is_legacy,
            add_declaration: self.add_declaration,
            add_deployment: self.add_deployment,
            sierra_max_bytecode_size: 180000,
            sierra_add_pythonic_hints: false,
            deployer_generate_salt: true,
            deployer_is_unique: true,
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

impl From<&ExpansionContext> for ExpansionContextFactory {
    fn from(value: &ExpansionContext) -> Self {
        ExpansionContextFactory::new(&value.contract_source)
            .with_contract_name(&value.contract_name)
            .with_derives(value.derives.iter().cloned())
            .with_contract_derives(value.contract_derives.iter().cloned())
            .with_execution(value.execution_version)
            .with_substitutions(value.substitutions.clone())
            .with_type_skips(value.type_skips.clone())
            .with_aliases(value.aliases.clone())
            .with_cainome_serde_path(&value.cainome_serde_path)
            .with_is_legacy(value.is_legacy)
            .with_root_module_path(value.root_module_path.clone())
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
