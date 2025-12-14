use std::collections::HashMap;

use proc_macro2::TokenStream;

use crate::expand::{utils, ExpansionResult, ROOT_MODULE_NAME};
use quote::quote;

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
        for module in self.submodules.into_values() {
            tokens.extend(module.to_token_stream())
        }

        let mut content = TokenStream::new();
        for content_module in self.content.into_values() {
            content.extend(content_module);
        }

        if self.name == ROOT_MODULE_NAME {
            quote! {
                #tokens

                #content
            }
        } else {
            let module_name = utils::str_to_ident(&self.name);
            quote! {
                pub mod #module_name {
                    #tokens

                    #content
                }
            }
        }
    }
}
