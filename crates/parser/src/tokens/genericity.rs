use syn::{GenericArgument, PathArguments, Type};

use super::Token;
use crate::CainomeResult;

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
    }
}
