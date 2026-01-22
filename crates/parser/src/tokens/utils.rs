/// Converts a snake case string to pascal case.
fn snake_to_pascal_case(s: &str) -> String {
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

// TODO(baitcode): need to find a better way. This method is only solving problems for the generic types inside tuples.
pub fn normalize_type_path(type_path: &str) -> String {
    type_path.to_string().replace(" ", "")
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
