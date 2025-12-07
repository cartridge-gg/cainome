use cainome_parser::tokens::Token;

use super::utils;

pub trait CairoToRust {
    fn to_rust_type(&self) -> String;

    fn to_rust_type_path(&self) -> String;
}

impl CairoToRust for &Token {
    fn to_rust_type(&self) -> String {
        match self {
            Token::Basic(t) => basic_types_to_rust(&t.type_name()),
            Token::Array(t) => {
                let internal_type = (&*t.inner.borrow()).to_rust_type();
                if t.is_legacy {
                    let ccsp = utils::cainome_cairo_serde_path();
                    format!("{}::CairoArrayLegacy<{}>", ccsp, internal_type)
                } else {
                    format!("Vec<{}>", internal_type)
                }
            }
            Token::Tuple(t) => {
                let mut s = String::from("(");

                for (idx, inner) in t.inners.iter().enumerate() {
                    let inner_type = (&*inner.borrow()).to_rust_type();
                    s.push_str(&inner_type);

                    if idx < t.inners.len() - 1 {
                        s.push_str(", ");
                    }
                }
                s.push(')');

                s
            }
            Token::Struct(c) => c.type_name(),
            Token::CompositeLegacy(c) => {
                let mut s = c.type_name_or_alias();

                let (type_name, is_builtin) = builtin_composite_to_rust(&s);
                if is_builtin {
                    s = type_name;
                }

                s
            }
            Token::Option(o) => format!("Option<{}>", (&*o.inner.borrow()).to_rust_type()),
            Token::Result(r) => format!(
                "Result<{}, {}>",
                (&*r.inner.borrow()).to_rust_type(),
                (&*r.error.borrow()).to_rust_type(),
            ),
            Token::NonZero(n) => {
                let ccsp = utils::cainome_cairo_serde_path();
                format!("{}::NonZero<{}>", ccsp, (&*n.inner.borrow()).to_rust_type())
            }
            _ => "__FUNCTION_NOT_SUPPORTED__".to_string(),
        }
    }

    fn to_rust_type_path(&self) -> String {
        match self {
            Token::Basic(t) => basic_types_to_rust(&t.type_name()),
            Token::Array(t) => {
                if t.is_legacy {
                    let ccsp = utils::cainome_cairo_serde_path();
                    format!(
                        "{}::CairoArrayLegacy::<{}>",
                        ccsp,
                        (&*t.inner.borrow()).to_rust_type_path()
                    )
                } else {
                    format!("Vec::<{}>", (&*t.inner.borrow()).to_rust_type_path())
                }
            }
            Token::Tuple(t) => {
                let mut s = String::from("(");
                for (idx, inner) in t.inners.iter().enumerate() {
                    s.push_str(&(&*inner.borrow()).to_rust_type_path());

                    if idx < t.inners.len() - 1 {
                        s.push_str(", ");
                    }
                }
                s.push(')');
                s
            }
            Token::CompositeLegacy(c) => {
                let mut s = c.type_name_or_alias();

                let (type_name, is_builtin) = builtin_composite_to_rust(&s);
                if is_builtin {
                    s = type_name;
                }

                s
            }
            Token::Option(o) => format!("Option::<{}>", (&*o.inner.borrow()).to_rust_type_path()),
            Token::Result(r) => format!(
                "Result::<{}, {}>",
                (&*r.inner.borrow()).to_rust_type_path(),
                (&*r.error.borrow()).to_rust_type_path(),
            ),
            Token::NonZero(n) => {
                let ccsp = utils::cainome_cairo_serde_path();
                format!(
                    "{}::NonZero::<{}>",
                    ccsp,
                    (&*n.inner.borrow()).to_rust_type_path()
                )
            }
            _ => "__FUNCTION_NOT_SUPPORTED__".to_string(),
        }
    }
}

fn basic_types_to_rust(type_name: &str) -> String {
    let ccsp = utils::cainome_cairo_serde_path();
    let snrs_types = utils::starknet_rs_types_path();

    match type_name {
        "ClassHash" => format!("{ccsp}::ClassHash"),
        "ContractAddress" => format!("{ccsp}::ContractAddress"),
        "EthAddress" => format!("{ccsp}::EthAddress"),
        "felt252" => format!("{snrs_types}::Felt"),
        "felt" => format!("{snrs_types}::Felt"),
        "bytes31" => format!("{ccsp}::Bytes31"),
        "ByteArray" => format!("{ccsp}::ByteArray"),
        "NonZero" => format!("{ccsp}::NonZero"),
        "U256" => format!("{ccsp}::U256"),
        _ => type_name.to_string(),
    }
}

fn builtin_composite_to_rust(type_name: &str) -> (String, bool) {
    let ccsp = utils::cainome_cairo_serde_path();
    let snrs_types = utils::starknet_rs_types_path();

    match type_name {
        "EthAddress" => (format!("{ccsp}::EthAddress"), true),
        "ByteArray" => (format!("{ccsp}::ByteArray"), true),
        "NonZero" => (format!("{ccsp}::NonZero"), true),
        "U256" => (format!("{ccsp}::U256"), true),
        // <https://github.com/starkware-libs/cairo/blob/35b299291fd7819f75409fb303ece7d30e4adb19/corelib/src/internal/bounded_int.cairo#L5>
        "BoundedInt" => (format!("{snrs_types}::Felt"), true),
        _ => (type_name.to_string(), false),
    }
}
