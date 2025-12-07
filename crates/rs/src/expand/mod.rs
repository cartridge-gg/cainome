pub(crate) mod contract;
pub(crate) mod enumeration;
pub(crate) mod event;
pub(crate) mod structure;
mod types;
pub(crate) mod utils;

use proc_macro2::TokenStream;
use quote::ToTokens;

use crate::ExecutionVersion;

#[derive(Clone)]
pub struct ExpansionContext {
    pub contract_name: String,
    pub derives: Vec<String>,
    pub execution_version: ExecutionVersion,
    pub type_param: Option<String>,
}

impl ExpansionContext {
    pub fn new(contract_name: &str) -> Self {
        Self {
            derives: vec![],
            contract_name: contract_name.to_string(),
            execution_version: crate::ExecutionVersion::V3,
            type_param: None,
        }
    }

    pub fn with_v1_execution(&self) -> Self {
        Self {
            execution_version: crate::ExecutionVersion::V1,
            ..self.clone()
        }
    }

    pub fn with_derives(&self, derives: Vec<&str>) -> Self {
        Self {
            derives: derives.iter().map(|i| i.to_string()).collect(),
            ..self.clone()
        }
    }

    pub fn with_type_param(&self, param: &str) -> Self {
        Self {
            type_param: Some(param.to_string()),
            ..self.clone()
        }
    }
}

pub trait Expandable {
    fn expand(&self, expansion_context: &ExpansionContext) -> Vec<TokenStream>;
}
