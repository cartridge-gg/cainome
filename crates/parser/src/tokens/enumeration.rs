use std::rc::Rc;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use crate::{
    abi::registry::TypeRegistry,
    tokens::{genericity, utils, NamedToken, Token},
    CainomeResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub type_path: String,
    variants: Vec<NamedToken>,
    pub generic_args: Vec<(String, Rc<RefCell<Token>>)>,
    pub fields_to_generics: HashMap<String, HashSet<String>>,
}

impl Enum {
    pub fn new(type_path: &str, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let Ok(generic_args_with_types) = generic_args
            .into_iter()
            .map(|(name, path)| registry.get(&path).map(|t| (name, t)))
            .collect::<Result<Vec<_>, _>>()
        else {
            unreachable!("Generic args should be registered in the registry");
        };

        Ok(Self {
            type_path: genericity::type_path_no_generic(&type_path),
            generic_args: generic_args_with_types,
            variants: vec![],
            fields_to_generics: HashMap::new(),
        })
    }

    pub fn with_variant(mut self, name: &str, token: Rc<RefCell<Token>>) -> Self {
        self.variants.push(NamedToken {
            name: name.to_string(),
            token: Rc::clone(&token),
        });
        for (generic_name, generic_token) in self.generic_args.iter() {
            if *token.borrow() == *generic_token.borrow() {
                let generic_candidates =
                    self.fields_to_generics.entry(name.to_string()).or_default();

                generic_candidates.insert(generic_name.to_owned());
            }
        }
        self
    }

    pub fn with_variants(self, variants: Vec<NamedToken>) -> Self {
        Self { variants, ..self }
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn is_generic(&self) -> bool {
        !self.generic_args.is_empty()
    }

    pub fn merge_generic_variant(&mut self, variant: &Enum) {
        for (field_name, candidates) in variant.fields_to_generics.iter() {
            let old_candidates = self
                .fields_to_generics
                .entry(field_name.to_owned())
                .or_default();
            let new_candidates = old_candidates.intersection(candidates).cloned().collect();
            *old_candidates = new_candidates;
        }
    }

    pub fn get_variants(&self) -> &Vec<NamedToken> {
        &self.variants
    }
}
