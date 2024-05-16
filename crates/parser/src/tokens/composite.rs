use super::constants::{CAIRO_COMPOSITE_BUILTINS, CAIRO_GENERIC_BUILTINS};
use super::genericity;
use super::Token;

use crate::CainomeResult;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompositeType {
    Struct,
    Enum,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompositeInnerKind {
    Key,
    Data,
    Nested,
    Flat,
    NotUsed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeInner {
    pub index: usize,
    pub name: String,
    pub kind: CompositeInnerKind,
    pub token: Token,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Composite {
    pub type_path: String,
    pub inners: Vec<CompositeInner>,
    pub generic_args: Vec<(String, Token)>,
    pub r#type: CompositeType,
    pub is_event: bool,
    pub alias: Option<String>,
}

impl Composite {
    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        let type_path = escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        Ok(Self {
            // We want to keep the path with generic for the generic resolution.
            type_path: type_path.to_string(),
            inners: vec![],
            generic_args,
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        })
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn is_generic(&self) -> bool {
        !self.generic_args.is_empty()
    }

    /// Returns true if the current composite is considered as Cairo builtin.
    /// This is useful to avoid expanding the structure if already managed by
    /// the backend (like Option and Result for instance).
    /// Spans and Arrays are handled by `array`.
    pub fn is_builtin(&self) -> bool {
        for b in CAIRO_GENERIC_BUILTINS {
            if self.type_path.starts_with(b) {
                return true;
            }
        }

        for b in CAIRO_COMPOSITE_BUILTINS {
            if self.type_path.starts_with(b) {
                return true;
            }
        }

        false
    }

    pub fn type_name(&self) -> String {
        // TODO: need to opti that with regex?
        extract_type_path_with_depth(&self.type_path_no_generic(), 0)
    }

    pub fn type_name_or_alias(&self) -> String {
        if let Some(a) = &self.alias {
            a.clone()
        } else {
            self.type_name()
        }
    }

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        if self.type_path_no_generic() == type_path {
            self.alias = Some(alias.to_string());
        }

        for ref mut i in &mut self.inners {
            if let Token::Composite(ref mut c) = i.token {
                c.apply_alias(type_path, alias);
            }
        }
    }

    pub fn resolve_generic(&self, generic_name: &str, generic_type_path: &str) -> Token {
        if self.type_path == generic_type_path {
            Token::GenericArg(generic_name.to_string())
        } else {
            let mut inners = vec![];

            for i in &self.inners {
                inners.push(CompositeInner {
                    index: i.index,
                    name: i.name.clone(),
                    token: i.token.resolve_generic(generic_name, generic_type_path),
                    kind: i.kind,
                })
            }

            Token::Composite(Self {
                type_path: self.type_path.clone(),
                generic_args: self.generic_args.clone(),
                inners,
                r#type: self.r#type,
                is_event: self.is_event,
                alias: None,
            })
        }
    }
}

/// Converts a snake case string to pascal case.
pub fn snake_to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

/// Escapes Rust keywords that may be found into cairo code.
pub fn escape_rust_keywords(s: &str) -> String {
    let keywords = ["move", "type", "final"];

    let mut s = s.to_string();

    for k in keywords {
        let k_start = format!("{k}::");
        let k_middle = format!("::{k}::");
        let k_end = format!("::{k}");

        if s == k {
            return format!("r#{k}");
        } else if s.starts_with(&k_start) {
            s = s.replace(&k_start, &format!("r#{k}::"));
        } else if s.ends_with(&k_end) {
            s = s.replace(&k_end, &format!("::r#{k}"));
        } else {
            s = s.replace(&k_middle, &format!("::r#{k}::"));
        }
    }

    s
}

/// Extracts the `type_path` with given module `depth`.
/// The extraction also converts all everything to `snake_case`.
///
/// # Arguments
///
/// * `type_path` - Type path to be extracted.
/// * `depth` - The module depth to extract.
///
/// # Examples
///
/// `module::module2::type_name` with depth 0 -> `TypeName`.
/// `module::module2::type_name` with depth 1 -> `Module2TypeName`.
/// `module::module2::type_name` with depth 2 -> `ModuleModule2TypeName`.
pub fn extract_type_path_with_depth(type_path: &str, depth: usize) -> String {
    let segments: Vec<&str> = type_path.split("::").collect();

    let mut depth = depth;
    if segments.len() < depth + 1 {
        depth = segments.len() - 1;
    }

    let segments = &segments[segments.len() - depth - 1..segments.len()];
    segments.iter().map(|s| snake_to_pascal_case(s)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::*;

    fn basic_felt252() -> Token {
        Token::CoreBasic(CoreBasic {
            type_path: "core::felt252".to_string(),
        })
    }

    fn basic_u64() -> Token {
        Token::CoreBasic(CoreBasic {
            type_path: "core::integer::u64".to_string(),
        })
    }

    fn array_felt252() -> Token {
        Token::Array(Array {
            type_path: "core::array::Array::<core::felt252>".to_string(),
            inner: Box::new(basic_felt252()),
            is_legacy: false,
        })
    }

    fn composite_simple() -> Token {
        Token::Composite(Composite {
            type_path: "module::MyStruct".to_string(),
            inners: vec![],
            generic_args: vec![],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        })
    }

    fn composite_with_generic() -> Token {
        Token::Composite(Composite {
            type_path: "module::MyStruct::<core::felt252>".to_string(),
            inners: vec![],
            generic_args: vec![("A".to_string(), basic_felt252())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        })
    }

    #[test]
    fn test_snake_to_pascal_case() {
        assert_eq!(snake_to_pascal_case("my_type"), "MyType");
        assert_eq!(snake_to_pascal_case("my_type_long"), "MyTypeLong");
        assert_eq!(snake_to_pascal_case("type"), "Type");
        assert_eq!(snake_to_pascal_case("MyType"), "MyType");
        assert_eq!(snake_to_pascal_case("MyType_hybrid"), "MyTypeHybrid");
        assert_eq!(snake_to_pascal_case(""), "");
    }

    #[test]
    fn test_extract_type_with_depth() {
        assert_eq!(extract_type_path_with_depth("type_name", 0), "TypeName");
        assert_eq!(extract_type_path_with_depth("type_name", 10), "TypeName");
        assert_eq!(
            extract_type_path_with_depth("module::TypeName", 1),
            "ModuleTypeName"
        );
        assert_eq!(
            extract_type_path_with_depth("module::TypeName", 8),
            "ModuleTypeName"
        );
        assert_eq!(
            extract_type_path_with_depth("module_one::module_1::TypeName", 2),
            "ModuleOneModule1TypeName"
        );
    }

    #[test]
    fn test_parse() {
        let expected = Composite {
            type_path: "module::MyStruct".to_string(),
            inners: vec![],
            generic_args: vec![],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        assert_eq!(Composite::parse("module::MyStruct").unwrap(), expected);
        assert!(!expected.is_generic());
    }

    #[test]
    fn test_parse_generic_one() {
        let expected = Composite {
            type_path: "module::MyStruct::<core::felt252>".to_string(),
            inners: vec![],
            generic_args: vec![("A".to_string(), basic_felt252())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        assert_eq!(
            Composite::parse("module::MyStruct::<core::felt252>").unwrap(),
            expected
        );
        assert!(expected.is_generic());
    }

    #[test]
    fn test_parse_generic_two() {
        let expected = Composite {
            type_path: "module::MyStruct::<core::felt252, core::integer::u64>".to_string(),
            inners: vec![],
            generic_args: vec![
                ("A".to_string(), basic_felt252()),
                ("B".to_string(), basic_u64()),
            ],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        assert_eq!(
            Composite::parse("module::MyStruct::<core::felt252, core::integer::u64>").unwrap(),
            expected
        );
        assert!(expected.is_generic());
    }

    #[test]
    fn test_type_name() {
        let mut c = Composite {
            type_path: "module::MyStruct".to_string(),
            inners: vec![],
            generic_args: vec![],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };
        assert_eq!(c.type_name(), "MyStruct");

        c.type_path = "module::MyStruct::<core::felt252>".to_string();
        assert_eq!(c.type_name(), "MyStruct");
    }

    #[test]
    fn test_resolve_generic() {
        let c = Composite {
            type_path: "module::MyStruct".to_string(),
            inners: vec![CompositeInner {
                index: 0,
                name: "field_1".to_string(),
                kind: CompositeInnerKind::NotUsed,
                token: basic_felt252(),
            }],
            generic_args: vec![("A".to_string(), basic_felt252())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        let t = c.resolve_generic("A", "core::felt252");
        let c_generic = t.to_composite().unwrap();

        assert_eq!(
            c_generic.inners[0].token,
            Token::GenericArg("A".to_string())
        );
    }

    #[test]
    fn test_resolve_generic_nested() {
        let c = Composite {
            type_path: "module::MyStruct".to_string(),
            inners: vec![CompositeInner {
                index: 0,
                name: "field_1".to_string(),
                kind: CompositeInnerKind::NotUsed,
                token: array_felt252(),
            }],
            generic_args: vec![("A".to_string(), basic_felt252())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        let t = c.resolve_generic("A", "core::felt252");
        let c_generic = t.to_composite().unwrap();

        assert_eq!(
            c_generic.inners[0].token,
            Token::Array(Array {
                type_path: "core::array::Array::<core::felt252>".to_string(),
                inner: Box::new(Token::GenericArg("A".to_string())),
                is_legacy: false,
            }),
        );
    }

    #[test]
    fn test_resolve_generic_array_generic() {
        let c = Composite {
            type_path: "module::MyStruct".to_string(),
            inners: vec![CompositeInner {
                index: 0,
                name: "field_1".to_string(),
                kind: CompositeInnerKind::NotUsed,
                token: array_felt252(),
            }],
            generic_args: vec![("A".to_string(), array_felt252())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        let t = c.resolve_generic("A", "core::array::Array::<core::felt252>");
        let c_generic = t.to_composite().unwrap();

        assert_eq!(
            c_generic.inners[0].token,
            Token::GenericArg("A".to_string()),
        );
    }

    #[test]
    fn test_resolve_generic_composite() {
        let c = Composite {
            type_path: "module::MyStructOutter".to_string(),
            inners: vec![CompositeInner {
                index: 0,
                name: "field_1".to_string(),
                kind: CompositeInnerKind::NotUsed,
                token: composite_simple(),
            }],
            generic_args: vec![("A".to_string(), composite_with_generic())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        let t = c.resolve_generic("A", "module::MyStruct");
        let c_generic = t.to_composite().unwrap();

        assert_eq!(
            c_generic.inners[0].token,
            Token::GenericArg("A".to_string()),
        );
    }

    #[test]
    fn test_resolve_generic_composite_generic() {
        let c = Composite {
            type_path: "module::MyStruct".to_string(),
            inners: vec![CompositeInner {
                index: 0,
                name: "field_1".to_string(),
                kind: CompositeInnerKind::NotUsed,
                token: composite_with_generic(),
            }],
            generic_args: vec![("A".to_string(), composite_with_generic())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        let t = c.resolve_generic("A", "module::MyStruct::<core::felt252>");
        let c_generic = t.to_composite().unwrap();

        assert_eq!(
            c_generic.inners[0].token,
            Token::GenericArg("A".to_string()),
        );
    }

    #[test]
    fn test_escape_rust_keywords() {
        assert_eq!(escape_rust_keywords("move"), "r#move",);

        assert_eq!(escape_rust_keywords("move::salut"), "r#move::salut",);

        assert_eq!(escape_rust_keywords("hey::move"), "hey::r#move",);

        assert_eq!(
            escape_rust_keywords("hey::move::salut"),
            "hey::r#move::salut",
        );

        assert_eq!(
            escape_rust_keywords("type::move::final"),
            "r#type::r#move::r#final",
        );
    }
}
