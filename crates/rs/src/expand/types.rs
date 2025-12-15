use std::rc::Rc;

use cainome_parser::tokens::{
    ArrayContainer, CoreBasic, Enum, Event, NamedToken, NonZeroContainer, OptionContainer,
    ResultContainer, Struct, Token, TupleContainer,
};

use crate::expand::{
    utils::{is_serde_hex_int, SerdeHexType},
    ExpansionContext,
};

use super::utils;

pub trait CairoToRust {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String;

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String;
}

impl CairoToRust for CoreBasic {
    fn to_rust_type(&self, _: &ExpansionContext) -> String {
        basic_types_to_rust(&self.type_name())
    }

    fn to_rust_type_path(&self, _: &ExpansionContext) -> String {
        basic_types_to_rust(&self.type_name())
    }
}

impl CairoToRust for ArrayContainer {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        let internal_type = (&*self.inner.borrow()).to_rust_type(ctx);
        if self.is_legacy {
            let ccsp = utils::cainome_cairo_serde_path();
            format!("{ccsp}::CairoArrayLegacy<{internal_type}>")
        } else {
            format!("Vec<{internal_type}>")
        }
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        if self.is_legacy {
            let ccsp = utils::cainome_cairo_serde_path();
            format!(
                "{ccsp}::CairoArrayLegacy::<{}>",
                (&*self.inner.borrow()).to_rust_type_path(ctx)
            )
        } else {
            format!("Vec::<{}>", (&*self.inner.borrow()).to_rust_type_path(ctx))
        }
    }
}

impl CairoToRust for OptionContainer {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        format!("Option<{}>", (&*self.inner.borrow()).to_rust_type(ctx))
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        format!(
            "Option::<{}>",
            (&*self.inner.borrow()).to_rust_type_path(ctx)
        )
    }
}

impl CairoToRust for ResultContainer {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        format!(
            "Result<{}, {}>",
            (&*self.inner.borrow()).to_rust_type(ctx),
            (&*self.error.borrow()).to_rust_type(ctx)
        )
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        format!(
            "Result::<{}, {}>",
            (&*self.inner.borrow()).to_rust_type_path(ctx),
            (&*self.error.borrow()).to_rust_type_path(ctx)
        )
    }
}

impl CairoToRust for NonZeroContainer {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        let ccsp = utils::cainome_cairo_serde_path();
        format!(
            "{ccsp}::NonZero<{}>",
            (&*self.inner.borrow()).to_rust_type(ctx)
        )
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        let ccsp = utils::cainome_cairo_serde_path();
        format!(
            "{ccsp}::NonZero::<{}>",
            (&*self.inner.borrow()).to_rust_type_path(ctx)
        )
    }
}

impl CairoToRust for TupleContainer {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        let mut s = String::from("(");

        for (idx, inner) in self.inners.iter().enumerate() {
            let inner_type = (&*inner.borrow()).to_rust_type(ctx);
            s.push_str(&inner_type);

            if idx < self.inners.len() - 1 {
                s.push_str(", ");
            }
        }
        s.push(')');

        s
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        let mut s = String::from("(");
        for (idx, inner) in self.inners.iter().enumerate() {
            s.push_str(&(&*inner.borrow()).to_rust_type_path(ctx));

            if idx < self.inners.len() - 1 {
                s.push_str(", ");
            }
        }
        s.push(')');
        s
    }
}

// TODO: rewrite
impl CairoToRust for Struct {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        [ctx.root_module_path.clone(), self.type_path_no_generic()].join("::")
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        [ctx.root_module_path.clone(), self.type_path_no_generic()].join("::")
    }
}

impl CairoToRust for Event {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        [ctx.root_module_path.clone(), self.type_path_no_generic()].join("::")
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        [ctx.root_module_path.clone(), self.type_path_no_generic()].join("::")
    }
}

impl CairoToRust for Enum {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        [ctx.root_module_path.clone(), self.type_path_no_generic()].join("::")
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        [ctx.root_module_path.clone(), self.type_path_no_generic()].join("::")
    }
}

impl CairoToRust for &Token {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        match self {
            Token::Basic(t) => t.to_rust_type(ctx),
            Token::Array(t) => t.to_rust_type(ctx),
            Token::Option(t) => t.to_rust_type(ctx),
            Token::Result(t) => t.to_rust_type(ctx),
            Token::NonZero(t) => t.to_rust_type(ctx),
            Token::Tuple(t) => t.to_rust_type(ctx),
            Token::Struct(t) => t.to_rust_type(ctx),
            Token::Event(t) => t.to_rust_type(ctx),
            Token::Enum(t) => t.to_rust_type(ctx),
            Token::Constructor(_) => "__CONSTRUCTOR_NOT_SUPPORTED__".to_string(),
            Token::Interface(_) => "__INTERFACE_NOT_SUPPORTED__".to_string(),
            Token::Function(_) => "__FUNCTION_NOT_SUPPORTED__".to_string(),
            Token::Placeholder => "__PLACEHOLDER__".to_string(),
        }
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        match self {
            Token::Basic(t) => t.to_rust_type_path(ctx),
            Token::Array(t) => t.to_rust_type_path(ctx),
            Token::Option(t) => t.to_rust_type_path(ctx),
            Token::Result(t) => t.to_rust_type_path(ctx),
            Token::NonZero(t) => t.to_rust_type_path(ctx),
            Token::Tuple(t) => t.to_rust_type_path(ctx),
            Token::Struct(t) => t.to_rust_type_path(ctx),
            Token::Event(t) => t.to_rust_type_path(ctx),
            Token::Enum(t) => t.to_rust_type_path(ctx),
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

pub fn extract_dependencies(items: &Vec<NamedToken>, ctx: &ExpansionContext) -> Vec<String> {
    let mut deps = vec![];

    // Unwrapping all container types to get inner types
    let unwrapped_types = items
        .iter()
        .flat_map(|named_token| match &*named_token.token.borrow() {
            Token::Array(t) => vec![Rc::clone(&t.inner)],
            Token::Option(t) => vec![Rc::clone(&t.inner)],
            Token::Result(t) => vec![Rc::clone(&t.inner), Rc::clone(&t.error)],
            Token::NonZero(t) => vec![Rc::clone(&t.inner)],
            Token::Tuple(t) => t.inners.iter().map(|el| Rc::clone(el)).collect(),
            // Composite types are added as is as those are imported from their modules
            // Basic types are also added but only to be skipped later
            _ => vec![Rc::clone(&named_token.token)],
        });

    for token in unwrapped_types {
        let inner_token = &*token.borrow();

        // Basic types are available through core library
        if inner_token.is_basic() {
            continue;
        }

        let type_path = inner_token.to_rust_type_path(ctx);

        let serde_hex = is_serde_hex_int(&type_path);
        if serde_hex != SerdeHexType::None {
            let ccsp = utils::cainome_cairo_serde_path();
            deps.push(ccsp);
        }

        deps.push(type_path);
    }

    return deps;
}
