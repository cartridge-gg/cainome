use syn::{GenericArgument, PathArguments, Type};

use super::Token;
use crate::CainomeResult;

/// Extracts the generic arguments from a type path.
///
/// # Arguments
///
/// * `type_path` - The type path to extract the generic arguments from.
///
/// # Returns
///
/// Returns a vector of tuples, where each tuple contains a string and a [`Token`].
/// The string is the name of the generic argument, starting to 'A' and incrementing
/// by 1 for each generic argument. The token is the token representing the generic
/// argument type.
pub fn extract_generics_args(type_path: &str) -> CainomeResult<Vec<(String, Token)>> {
    let t: Type = syn::parse_str(type_path)?;

    let mut generic_args = vec![];

    if let Type::Path(p) = t {
        if let Some(segment) = p.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                // Starts to 'A' for generic arguments.
                let ascii: u8 = 65;
                let mut i = 0;

                for arg in &args.args {
                    if let GenericArgument::Type(ty) = arg {
                        let arg_name = ((ascii + i as u8) as char).to_string();
                        let arg_str = quote::quote!(#ty).to_string().replace(' ', "");
                        generic_args.push((arg_name, Token::parse(&arg_str)?));
                        i += 1;
                    }
                }
            }
        }
    }

    Ok(generic_args)
}

/// Returns the type path without any generic arguments.
///
/// # Arguments
///
/// * `type_path` - The type path to remove the generic arguments from.
///
/// # Returns
///
/// Returns the type path without any generic arguments.
pub fn type_path_no_generic(type_path: &str) -> String {
    let frags: Vec<&str> = type_path.split('<').collect();
    frags
        .first()
        .unwrap_or(&type_path)
        .trim_end_matches("::")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_type_with_depth() {
        assert_eq!(type_path_no_generic("TypeName"), "TypeName");
        assert_eq!(type_path_no_generic("module::TypeName"), "module::TypeName");
        assert_eq!(type_path_no_generic("TypeName<core::felt252>"), "TypeName");
        assert_eq!(
            type_path_no_generic("module::TypeName<core::integer::u64>"),
            "module::TypeName"
        );
        assert_eq!(type_path_no_generic("TypeName<core::felt252, core::bool>"), "TypeName");
    }

    #[test]
    fn test_extract_generics_args_no_generic() {
        let generics_args = extract_generics_args("module::TypeName").unwrap();
        assert_eq!(generics_args.len(), 0);
    }

    #[test]
    fn test_extract_generics_args_single() {
        let generics_args = extract_generics_args("module::TypeName::<core::felt252>").unwrap();
        assert_eq!(generics_args.len(), 1);
        assert_eq!(generics_args[0].0, "A");
        assert_eq!(generics_args[0].1, Token::parse("core::felt252").unwrap());
    }

    #[test]
    fn test_extract_generics_args_multiple() {
        let generics_args = extract_generics_args("module::TypeName::<core::felt252, core::bool>").unwrap();
        assert_eq!(generics_args.len(), 2);
        assert_eq!(generics_args[0].0, "A");
        assert_eq!(generics_args[0].1, Token::parse("core::felt252").unwrap());
        assert_eq!(generics_args[1].0, "B");
        assert_eq!(generics_args[1].1, Token::parse("core::bool").unwrap());
    }
}
