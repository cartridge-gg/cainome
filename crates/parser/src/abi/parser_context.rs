use std::collections::{HashMap, HashSet};

use crate::tokens::genericity;

#[derive(Clone, Default)]
pub struct ParserContext {
    pub substitutions: HashMap<String, String>,
    pub type_skips: HashSet<String>,
}

impl ParserContext {
    pub fn new() -> Self {
        Self {
            substitutions: HashMap::new(),
            type_skips: HashSet::new(),
        }
    }

    pub fn with_substitutions(mut self, substitutions: HashMap<&str, &str>) -> Self {
        self.substitutions = substitutions
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        self
    }

    pub fn with_type_skips<S>(mut self, type_skips: Vec<S>) -> Self
    where
        S: AsRef<str>,
    {
        for skip in type_skips.iter() {
            self.type_skips.insert(skip.as_ref().to_string());
            self.type_skips.insert(skip.as_ref().to_string());
        }
        self
    }

    pub fn is_type_skipped(&self, type_path: &str) -> bool {
        let type_path_no_generic = genericity::type_path_no_generic(type_path);
        self.type_skips.contains(type_path) || self.type_skips.contains(&type_path_no_generic)
    }
}
