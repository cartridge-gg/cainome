use std::collections::HashMap;

use proc_macro2::TokenStream;
use syn::token::Mod;

use crate::ExecutionVersion;
use quote::quote;

pub(crate) mod contract;
pub(crate) mod enumeration;
pub(crate) mod event;
pub(crate) mod structure;
mod types;
pub(crate) mod utils;

pub const ROOT_MODULE_NAME: &str = "";

#[derive(Clone)]
pub struct ExpansionResult {
    pub name: String,
    pub path: Vec<String>,
    pub content: HashMap<String, TokenStream>,
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
            }
        } else {
            Self {
                name: full_path.to_string(),
                path: vec![],
                content: HashMap::new(),
            }
        };

        res
    }

    pub fn with_item(mut self, item_name: &str, item: TokenStream) -> Self {
        self.content.insert(item_name.to_string(), item);
        self
    }
}

pub struct Module {
    pub name: String,
    pub submodules: HashMap<String, Module>,
    pub content: HashMap<String, TokenStream>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            name: ROOT_MODULE_NAME.to_string(),
            submodules: HashMap::new(),
            content: HashMap::new(),
        }
    }

    pub fn register(&mut self, result: ExpansionResult) {
        let mut current_module = self;

        for segment in &result.path {
            current_module = current_module
                .submodules
                .entry(segment.to_string())
                .or_insert(Module {
                    name: segment.to_string(),
                    submodules: HashMap::new(),
                    content: HashMap::new(),
                });
        }

        if current_module.content.contains_key(&result.name) {
            return;
        }

        let module_content = result.content.into_values().collect::<Vec<_>>();

        let module_content = quote! {
            #(#module_content)*
        };

        current_module.content.insert(result.name, module_content);
    }

    pub fn with_registered_many(mut self, results: Vec<ExpansionResult>) -> Self {
        self.register_many(results);
        self
    }

    pub fn register_many(&mut self, results: Vec<ExpansionResult>) {
        for result in results {
            self.register(result);
        }
    }

    pub fn to_token_stream(self) -> TokenStream {
        let mut tokens = TokenStream::new();

        // Flatten modules
        for module in self.submodules.values() {
            let mod_name = utils::str_to_type(&module.name);
            let mod_content = module.content.values();

            tokens.extend(if module.name == ROOT_MODULE_NAME {
                quote! {
                    #(#mod_content)*
                }
            } else {
                quote! {
                    pub mod #mod_name {
                        #(#mod_content)*
                    }
                }
            })
        }

        tokens.extend(self.content.into_values());

        tokens
    }
}

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
    // pub modules: HashMap<String, Module>,
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
    fn expand(&self, ctx: &ExpansionContext) -> Vec<ExpansionResult>;
}
