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

pub const ROOT_MODULE_NAME: &str = "ROOT";

#[derive(Clone)]
pub struct Module {
    pub name: String,
    pub content: HashMap<String, TokenStream>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            content: HashMap::new(),
        }
    }

    pub fn add_item(mut self, item_name: &str, item: TokenStream) -> Self {
        self.content.insert(item_name.to_string(), item);
        self
    }

    pub fn merge(&mut self, other: Module) {
        if self.name != other.name {
            return;
        }

        for (key, value) in other.content {
            if self.content.contains_key(&key) {
                continue;
            }

            self.content.insert(key, value);
        }
    }
}

pub struct Modules {
    pub modules: HashMap<String, Module>,
}

impl Modules {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: Module) {
        if let Some(existing_module) = self.modules.get_mut(&module.name) {
            existing_module.merge(module);
        } else {
            self.modules.insert(module.name.clone(), module);
        }
    }

    pub fn register_many(&mut self, modules: Vec<Module>) {
        for module in modules {
            self.register(module);
        }
    }

    pub fn to_token_stream(self) -> TokenStream {
        render(self.modules.into_values().collect())
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
    // pub fn add_token_stream(&mut self, path: &str, value: TokenStream) {
    //     let mut segments = path.split("::").collect::<Vec<_>>();
    //     if segments.is_empty() {
    //         return;
    //     }

    //     let type_name = segments.pop().unwrap();
    //     let module_path = segments.join("::");

    //     let module = self.modules.entry(module_path.clone()).or_insert(Module {
    //         name: module_path,
    //         content: HashMap::new(),
    //     });

    //     module.content.insert(type_name.to_string(), value);
    // }

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
    fn expand(&self, ctx: &ExpansionContext) -> Vec<Module>;
}

// TODO: naming!
pub fn render(modules: Vec<Module>) -> TokenStream {
    let mut tokens = TokenStream::new();

    // Flatten modules
    for module in modules.iter() {
        let mod_name = utils::str_to_ident(&module.name);
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

    tokens
}
