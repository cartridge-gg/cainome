use std::collections::{BTreeMap, HashMap, HashSet};

use proc_macro2::TokenStream;

use crate::expand::{utils, ExpansionResult, ROOT_MODULE_NAME};
use quote::{quote, ToTokens};

#[derive(Clone)]
pub struct Module {
    pub name: String,
    pub imports: HashSet<syn::UseTree>,
    pub submodules: BTreeMap<String, Module>,
    pub content: HashMap<String, TokenStream>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            name: ROOT_MODULE_NAME.to_string(),
            imports: HashSet::new(),
            submodules: BTreeMap::new(),
            content: HashMap::new(),
        }
    }

    pub fn include(
        &mut self,
        result: ExpansionResult,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let mut current_module = self;

        for segment in &result.path {
            current_module = current_module
                .submodules
                .entry(segment.to_string())
                .or_insert(Module {
                    name: segment.to_string(),
                    submodules: BTreeMap::new(),
                    content: HashMap::new(),
                    imports: HashSet::new(),
                });
        }

        if current_module.content.contains_key(&result.name) {
            return Ok(());
        }

        let parsed_imports = result
            .imports
            .iter()
            .map(|i| syn::parse_str::<syn::UseTree>(&i))
            .collect::<Result<Vec<_>, _>>();

        current_module.imports.extend(parsed_imports?);

        let module_content = result.content.into_values().collect::<Vec<_>>();

        let module_content = quote! {
            #(#module_content)*
        };

        current_module.content.insert(result.name, module_content);
        Ok(())
    }

    pub fn with_includes(
        mut self,
        results: Vec<ExpansionResult>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static>> {
        self.include_many(results)?;
        Ok(self)
    }

    pub fn include_many(
        &mut self,
        results: Vec<ExpansionResult>,
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        for result in results {
            self.include(result)?;
        }
        Ok(())
    }

    pub fn token_stream(self) -> TokenStream {
        let mut tokens = TokenStream::new();

        let mut modules = self.submodules.into_values().collect::<Vec<_>>();
        modules.sort_by_key(|i| i.name.clone());

        // Flatten modules
        for module in modules {
            tokens.extend(module.token_stream())
        }

        let mut content = TokenStream::new();
        for content_module in self.content.into_values() {
            content.extend(content_module);
        }

        let imports = self
            .imports
            .into_iter()
            .map(|i| {
                quote! {
                    use #i;
                }
            })
            .collect::<Vec<_>>();

        if self.name == ROOT_MODULE_NAME {
            quote! {
                #(#imports)*

                #tokens

                #content
            }
        } else {
            let module_name = utils::str_to_ident(&self.name);
            quote! {
                pub mod #module_name {
                    #(#imports)*

                    #tokens

                    #content
                }
            }
        }
    }
}

impl ToTokens for Module {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.clone().token_stream());
    }
}
