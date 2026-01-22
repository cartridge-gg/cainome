use std::rc::Rc;

use cainome_parser::tokens::{
    ArrayContainer, Enum, Event, NamedToken, NonZeroContainer, OptionContainer, ResultContainer,
    Struct, Token, TupleContainer, TypePath,
};

use crate::expand::{
    utils::{is_serde_hex_int, SerdeHexType},
    ExpansionContext,
};

pub trait CairoToRust {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String;

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String;
}

impl CairoToRust for TypePath {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        ctx.apply_alias(&self.type_path)
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        ctx.apply_alias(&self.type_path)
    }
}

impl CairoToRust for ArrayContainer {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        let internal_type = (&*self.inner.borrow()).to_rust_type(ctx);
        if ctx.is_legacy {
            let ccsp = ctx.cainome_serde_path.to_string();
            format!("{ccsp}::CairoArrayLegacy<{internal_type}>")
        } else {
            format!("Vec<{internal_type}>")
        }
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        if ctx.is_legacy {
            let ccsp = ctx.cainome_serde_path.to_string();
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
        let ccsp = ctx.cainome_serde_path.to_string();
        format!(
            "{ccsp}::NonZero<{}>",
            (&*self.inner.borrow()).to_rust_type(ctx)
        )
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        let ccsp = ctx.cainome_serde_path.to_string();
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

impl CairoToRust for Struct {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        [
            ctx.root_module_path.clone(),
            ctx.apply_alias(&self.type_path),
        ]
        .join("::")
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        [
            ctx.root_module_path.clone(),
            ctx.apply_alias(&self.type_path),
        ]
        .join("::")
    }
}

impl CairoToRust for Event {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        [
            ctx.root_module_path.clone(),
            ctx.apply_alias(&self.type_path),
        ]
        .join("::")
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        [
            ctx.root_module_path.clone(),
            ctx.apply_alias(&self.type_path),
        ]
        .join("::")
    }
}

impl CairoToRust for Enum {
    fn to_rust_type(&self, ctx: &ExpansionContext) -> String {
        [
            ctx.root_module_path.clone(),
            ctx.apply_alias(&self.type_path_no_generic()),
        ]
        .join("::")
    }

    fn to_rust_type_path(&self, ctx: &ExpansionContext) -> String {
        [
            ctx.root_module_path.clone(),
            ctx.apply_alias(&self.type_path_no_generic()),
        ]
        .join("::")
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
            Token::Substitute(t) => t.to_rust_type(ctx),
            Token::Skip(t) => t.to_rust_type(ctx),
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
            Token::Skip(t) => t.to_rust_type_path(ctx),
            Token::Substitute(t) => t.to_rust_type_path(ctx),
        }
    }
}

fn check_requires_serde_derive(items: &[NamedToken], ctx: &ExpansionContext) -> bool {
    // Unwrapping all container types to get inner types
    let unwrapped_types = items
        .iter()
        .flat_map(|named_token| match &*named_token.token.borrow() {
            Token::Array(t) => vec![Rc::clone(&t.inner)],
            Token::Option(t) => vec![Rc::clone(&t.inner)],
            Token::Result(t) => vec![Rc::clone(&t.inner), Rc::clone(&t.error)],
            Token::NonZero(t) => vec![Rc::clone(&t.inner)],
            Token::Tuple(t) => t.inners.iter().map(Rc::clone).collect(),
            // Composite types are added as is as those are imported from their modules
            // Basic types are also added but only to be skipped later
            _ => vec![Rc::clone(&named_token.token)],
        });

    for token in unwrapped_types {
        let inner_token = &*token.borrow();

        let type_path = inner_token.to_rust_type_path(ctx);

        let serde_hex = is_serde_hex_int(&type_path);
        if serde_hex != SerdeHexType::None {
            return true;
        }
    }

    false
}

pub fn get_additional_derive_requirements(
    items: &[NamedToken],
    ctx: &ExpansionContext,
) -> Vec<String> {
    if check_requires_serde_derive(items, ctx) {
        return vec![
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
        ];
    }
    vec![]
}
