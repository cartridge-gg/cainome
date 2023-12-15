use cainome_parser::tokens::Token;

use super::utils;

pub trait CairoToRust {
    fn to_rust_type(&self) -> String;

    fn to_rust_type_path(&self) -> String;
}

impl CairoToRust for Token {
    fn to_rust_type(&self) -> String {
        match self {
            Token::CoreBasic(t) => basic_types_to_rust(&t.type_name()),
            Token::Array(t) => format!("Vec<{}>", t.inner.to_rust_type()),
            Token::Tuple(t) => {
                let mut s = String::from("(");

                for (idx, inner) in t.inners.iter().enumerate() {
                    s.push_str(&inner.to_rust_type());

                    if idx < t.inners.len() - 1 {
                        s.push_str(", ");
                    }
                }
                s.push(')');

                s
            }
            Token::Composite(c) => c.type_name_or_alias(),
            Token::GenericArg(s) => s.clone(),
            _ => "__FUNCTION_NOT_SUPPORTED__".to_string(),
        }
    }

    fn to_rust_type_path(&self) -> String {
        match self {
            Token::CoreBasic(t) => basic_types_to_rust(&t.type_name()),
            Token::Array(t) => format!("Vec::<{}>", t.inner.to_rust_type_path()),
            Token::Tuple(t) => {
                let mut s = String::from("(");
                for (idx, inner) in t.inners.iter().enumerate() {
                    s.push_str(&inner.to_rust_type_path());

                    if idx < t.inners.len() - 1 {
                        s.push_str(", ");
                    }
                }
                s.push(')');
                s
            }
            Token::Composite(c) => {
                let mut s = c.type_name_or_alias();

                if c.is_generic() {
                    s.push_str("::<");
                    for (i, (_, token)) in c.generic_args.iter().enumerate() {
                        s.push_str(&token.to_rust_type_path());
                        if i < c.generic_args.len() - 1 {
                            s.push(',');
                        }
                    }
                    s.push('>');
                }

                s
            }
            Token::GenericArg(s) => s.clone(),
            _ => "__FUNCTION_NOT_SUPPORTED__".to_string(),
        }
    }
}

fn basic_types_to_rust(type_name: &str) -> String {
    let ccsp = utils::cainome_cairo_serde_path();

    match type_name {
        "ClassHash" => format!("{ccsp}::ClassHash"),
        "ContractAddress" => format!("{ccsp}::ContractAddress"),
        "EthAddress" => format!("{ccsp}::EthAddress"),
        "felt252" => "starknet::core::types::FieldElement".to_string(),
        _ => type_name.to_string(),
    }
}
