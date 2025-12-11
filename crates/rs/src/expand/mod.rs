use proc_macro2::TokenStream;

use crate::ExecutionVersion;

pub(crate) mod contract;
pub(crate) mod enumeration;
pub(crate) mod event;
pub(crate) mod structure;
mod types;
pub(crate) mod utils;

#[allow(dead_code)]
#[derive(Clone)]
pub struct ExpansionContext {
    pub contract_name: String,
    pub derives: Vec<String>,
    pub execution_version: ExecutionVersion,
    // TODO: move into enum expansion context?
    pub type_param: Option<String>,
    pub outer_enum: Option<String>,
    pub variant_name: Option<String>,
}

#[allow(dead_code)]
impl ExpansionContext {
    pub fn new(contract_name: &str) -> Self {
        Self {
            derives: vec![],
            contract_name: contract_name.to_string(),
            execution_version: crate::ExecutionVersion::V3,
            type_param: None,
            outer_enum: None,
            variant_name: None,
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

    pub fn with_outer_enum(&self, enum_name: &str) -> Self {
        Self {
            outer_enum: Some(enum_name.to_string()),
            ..self.clone()
        }
    }

    pub fn with_variant_name(&self, variant_name: &str) -> Self {
        Self {
            variant_name: Some(variant_name.to_string()),
            ..self.clone()
        }
    }
}

pub trait Expandable {
    fn expand(&self, expansion_context: &ExpansionContext) -> TokenStream;
}
