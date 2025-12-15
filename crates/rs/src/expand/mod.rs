use std::collections::{HashMap, HashSet};

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
pub(crate) mod utils;

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

#[allow(dead_code)]
#[derive(Clone)]
pub struct ExpansionContext {
    pub contract_name: String,
    pub derives: Vec<String>,
    pub execution_version: ExecutionVersion,
    // TODO: syn::Type?
    pub root_module_path: String,
}

#[allow(dead_code)]
impl ExpansionContext {
    pub fn new(contract_name: &str) -> Self {
        Self {
            derives: vec![],
            contract_name: contract_name.to_string(),
            execution_version: crate::ExecutionVersion::V3,
            root_module_path: "crate".to_string(),
        }
    }
    pub fn with_execution(self, execution_version: ExecutionVersion) -> Self {
        Self {
            execution_version: execution_version,
            ..self.clone()
        }
    }

    pub fn with_v1_execution(self) -> Self {
        self.with_execution(ExecutionVersion::V1)
    }

    pub fn with_derives<I, S>(self, derives: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            derives: derives.into_iter().map(|s| s.as_ref().to_owned()).collect(),
            ..self.clone()
        }
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
