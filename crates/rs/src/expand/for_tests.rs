use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    visit::{self, Visit},
    File, ItemStruct, Stmt,
};
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

fn colorize_rust(code: &str) -> String {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let mut out = String::new();
    for line in code.lines() {
        let ranges = h.highlight_line(line, &ps).unwrap();
        out.push_str(&syntect::util::as_24_bit_terminal_escaped(
            &ranges[..],
            true,
        ));
        out.push('\n');
    }
    out
}

pub fn output_pretty(code: &TokenStream) {
    let generated_ast: &File = &syn::parse2(code.to_token_stream()).unwrap();
    let s = prettyplease::unparse(generated_ast);
    tracing::trace!("{}", colorize_rust(&s));
}

pub fn assert_code_has<T: ToTokens>(generated: &T, expected: &T, message: &str) {
    let generated_ast = &syn::parse2(generated.to_token_stream()).unwrap();
    let expected_ast = &syn::parse2(expected.to_token_stream()).unwrap();

    let normalised_generated = prettyplease::unparse(generated_ast);
    let normalised_expected = prettyplease::unparse(expected_ast);

    let normalised_generated_tokens = TokenStream::from_str(&normalised_generated)
        .unwrap()
        .to_string();
    let normalised_expected_tokens = TokenStream::from_str(&normalised_expected)
        .unwrap()
        .to_string();

    let has_match = normalised_generated_tokens.contains(&normalised_expected_tokens);

    assert!(
        has_match,
        "{}. Expected to find:\n\n{}\n\nInside of: \n\n{}",
        message,
        colorize_rust(&normalised_expected),
        colorize_rust(&normalised_generated)
    );
}

pub fn assert_code_has_statement<T: ToTokens>(generated: &T, expected: &Stmt, message: &str) {
    struct Finder<'ast> {
        pub expected: &'ast Stmt,
        pub found: bool,
    }

    impl<'ast> Finder<'ast> {
        fn new(stmt: &'ast Stmt) -> Self {
            Self {
                expected: stmt,
                found: false,
            }
        }
    }

    impl<'ast> Visit<'ast> for Finder<'ast> {
        fn visit_stmt(&mut self, node: &'ast Stmt) {
            if node == self.expected {
                self.found = true;
            } else {
                visit::visit_stmt(self, node);
            }
        }
    }

    let mut finder = Finder::new(expected);

    let generated_ast = &syn::parse2(generated.to_token_stream()).unwrap();
    let normalised_generated = prettyplease::unparse(generated_ast);

    finder.visit_file(generated_ast);

    assert!(
        finder.found,
        "{}. Expected to find statement:\n\n{}\n\nInside of: \n\n{}",
        message,
        &expected.to_token_stream(),
        colorize_rust(&normalised_generated)
    );
}

fn find_struct_in_file(generated: &File, expected: &ItemStruct) -> bool {
    struct Finder<'ast> {
        pub expected: &'ast ItemStruct,
        pub found: bool,
    }

    impl<'ast> Finder<'ast> {
        fn new(strct: &'ast ItemStruct) -> Self {
            Self {
                expected: strct,
                found: false,
            }
        }
    }

    impl<'ast> Visit<'ast> for Finder<'ast> {
        fn visit_item_struct(&mut self, strct: &'ast syn::ItemStruct) {
            if strct == self.expected {
                self.found = true;
            } else {
                visit::visit_item_struct(self, strct);
            }
        }
    }

    let mut finder = Finder::new(expected);

    finder.visit_file(generated);

    finder.found
}

pub fn assert_code_has_struct<T: ToTokens>(generated: &T, expected: &ItemStruct, message: &str) {
    let generated_ast = &syn::parse2(generated.to_token_stream()).unwrap();
    let found = find_struct_in_file(generated_ast, expected);

    assert!(
        found,
        "{}. Expected to find statement:\n\n{}\n\nInside of: \n\n{}",
        message,
        &expected.to_token_stream(),
        generated_ast.to_token_stream()
    );
}

pub fn assert_code_has_not_struct<T: ToTokens>(
    generated: &T,
    expected: &ItemStruct,
    message: &str,
) {
    let generated_ast = &syn::parse2(generated.to_token_stream()).unwrap();
    let found = find_struct_in_file(generated_ast, expected);

    assert!(
        !found,
        "{}. Expected not to find statement:\n\n{}\n\nInside of: \n\n{}",
        message,
        &expected.to_token_stream(),
        generated_ast.to_token_stream()
    );
}

pub fn assert_code_has_not<T: ToTokens>(generated: &TokenStream, expected: &T, message: &str) {
    let generated_ast: &File = &syn::parse2(generated.to_token_stream()).unwrap();
    let expected_ast = &syn::parse2(expected.to_token_stream()).unwrap();

    let normalised_generated = prettyplease::unparse(generated_ast);
    let normalised_expected = prettyplease::unparse(expected_ast);

    let normalised_generated_tokens = TokenStream::from_str(&normalised_generated)
        .unwrap()
        .to_string();
    let normalised_expected_tokens = TokenStream::from_str(&normalised_expected)
        .unwrap()
        .to_string();

    let has_match = normalised_generated_tokens.contains(&normalised_expected_tokens);

    assert!(
        !has_match,
        "{}. Expected not to find:\n\n{}\n\nInside of: \n\n{}",
        message,
        colorize_rust(&normalised_expected),
        colorize_rust(&normalised_generated)
    );
}
