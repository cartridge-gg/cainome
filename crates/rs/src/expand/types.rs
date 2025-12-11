use cainome_parser::tokens::{
    ArrayContainer, CoreBasic, Enum, Event, NonZeroContainer, OptionContainer, ResultContainer,
    Struct, Token, TupleContainer,
};

use super::utils;

pub trait CairoToRust {
    fn to_rust_type(&self) -> String;

    fn to_rust_type_path(&self) -> String;
}
impl CairoToRust for CoreBasic {
    fn to_rust_type(&self) -> String {
        basic_types_to_rust(&self.type_name())
    }

    fn to_rust_type_path(&self) -> String {
        basic_types_to_rust(&self.type_name())
    }
}
impl CairoToRust for ArrayContainer {
    fn to_rust_type(&self) -> String {
        let internal_type = (&*self.inner.borrow()).to_rust_type();
        if self.is_legacy {
            let ccsp = utils::cainome_cairo_serde_path();
            format!("{ccsp}::CairoArrayLegacy<{internal_type}>")
        } else {
            format!("Vec<{internal_type}>")
        }
    }

    fn to_rust_type_path(&self) -> String {
        if self.is_legacy {
            let ccsp = utils::cainome_cairo_serde_path();
            format!(
                "{ccsp}::CairoArrayLegacy::<{}>",
                (&*self.inner.borrow()).to_rust_type_path()
            )
        } else {
            format!("Vec::<{}>", (&*self.inner.borrow()).to_rust_type_path())
        }
    }
}
impl CairoToRust for OptionContainer {
    fn to_rust_type(&self) -> String {
        format!("Option<{}>", (&*self.inner.borrow()).to_rust_type())
    }

    fn to_rust_type_path(&self) -> String {
        format!("Option::<{}>", (&*self.inner.borrow()).to_rust_type_path())
    }
}

impl CairoToRust for ResultContainer {
    fn to_rust_type(&self) -> String {
        format!(
            "Result<{}, {}>",
            (&*self.inner.borrow()).to_rust_type(),
            (&*self.error.borrow()).to_rust_type()
        )
    }

    fn to_rust_type_path(&self) -> String {
        format!(
            "Result::<{}, {}>",
            (&*self.inner.borrow()).to_rust_type_path(),
            (&*self.error.borrow()).to_rust_type_path()
        )
    }
}

impl CairoToRust for NonZeroContainer {
    fn to_rust_type(&self) -> String {
        let ccsp = utils::cainome_cairo_serde_path();
        format!(
            "{ccsp}::NonZero<{}>",
            (&*self.inner.borrow()).to_rust_type()
        )
    }

    fn to_rust_type_path(&self) -> String {
        let ccsp = utils::cainome_cairo_serde_path();
        format!(
            "{ccsp}::NonZero::<{}>",
            (&*self.inner.borrow()).to_rust_type_path()
        )
    }
}

impl CairoToRust for TupleContainer {
    fn to_rust_type(&self) -> String {
        let mut s = String::from("(");

        for (idx, inner) in self.inners.iter().enumerate() {
            let inner_type = (&*inner.borrow()).to_rust_type();
            s.push_str(&inner_type);

            if idx < self.inners.len() - 1 {
                s.push_str(", ");
            }
        }
        s.push(')');

        s
    }

    fn to_rust_type_path(&self) -> String {
        let mut s = String::from("(");
        for (idx, inner) in self.inners.iter().enumerate() {
            s.push_str(&(&*inner.borrow()).to_rust_type_path());

            if idx < self.inners.len() - 1 {
                s.push_str(", ");
            }
        }
        s.push(')');
        s
    }
}

impl CairoToRust for Struct {
    fn to_rust_type(&self) -> String {
        self.type_name()
    }

    fn to_rust_type_path(&self) -> String {
        self.type_name()
    }
}

impl CairoToRust for Event {
    fn to_rust_type(&self) -> String {
        self.type_name()
    }

    fn to_rust_type_path(&self) -> String {
        self.type_name()
    }
}

impl CairoToRust for Enum {
    fn to_rust_type(&self) -> String {
        self.type_name()
    }

    fn to_rust_type_path(&self) -> String {
        self.type_name()
    }
}

impl CairoToRust for &Token {
    fn to_rust_type(&self) -> String {
        match self {
            Token::Basic(t) => t.to_rust_type(),
            Token::Array(t) => t.to_rust_type(),
            Token::Option(t) => t.to_rust_type(),
            Token::Result(t) => t.to_rust_type(),
            Token::NonZero(t) => t.to_rust_type(),
            Token::Tuple(t) => t.to_rust_type(),
            Token::Struct(t) => t.to_rust_type(),
            Token::Event(t) => t.to_rust_type(),
            Token::Enum(t) => t.to_rust_type(),
            Token::Constructor(_) => "__CONSTRUCTOR_NOT_SUPPORTED__".to_string(),
            Token::Interface(_) => "__INTERFACE_NOT_SUPPORTED__".to_string(),
            Token::Function(_) => "__FUNCTION_NOT_SUPPORTED__".to_string(),
            Token::Placeholder => "__PLACEHOLDER__".to_string(),
        }
    }

    fn to_rust_type_path(&self) -> String {
        match self {
            Token::Basic(t) => t.to_rust_type_path(),
            Token::Array(t) => t.to_rust_type_path(),
            Token::Option(t) => t.to_rust_type_path(),
            Token::Result(t) => t.to_rust_type_path(),
            Token::NonZero(t) => t.to_rust_type_path(),
            Token::Tuple(t) => t.to_rust_type_path(),
            Token::Struct(t) => t.to_rust_type_path(),
            Token::Event(t) => t.to_rust_type_path(),
            Token::Enum(t) => t.to_rust_type_path(),
            Token::Function(_) => "__FUNCTION_NOT_SUPPORTED__".to_string(),
            Token::Interface(_) => "__INTERFACE_NOT_SUPPORTED__".to_string(),
            Token::Constructor(_) => "__CONSTRUCTOR_NOT_SUPPORTED__".to_string(),
            Token::Placeholder => "__PLACEHOLDER__".to_string(),
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

// fn builtin_composite_to_rust(type_name: &str) -> (String, bool) {
//     let ccsp = utils::cainome_cairo_serde_path();
//     let snrs_types = utils::starknet_rs_types_path();

//     match type_name {
//         "EthAddress" => (format!("{ccsp}::EthAddress"), true),
//         "ByteArray" => (format!("{ccsp}::ByteArray"), true),
//         "NonZero" => (format!("{ccsp}::NonZero"), true),
//         "U256" => (format!("{ccsp}::U256"), true),
//         // <https://github.com/starkware-libs/cairo/blob/35b299291fd7819f75409fb303ece7d30e4adb19/corelib/src/internal/bounded_int.cairo#L5>
//         "BoundedInt" => (format!("{snrs_types}::Felt"), true),
//         _ => (type_name.to_string(), false),
//     }
// }
