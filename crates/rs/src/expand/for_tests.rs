use proc_macro2::TokenStream;
use quote::ToTokens;

pub fn assert_code_has<T: ToTokens>(generated: &TokenStream, expected: &T, message: &str) {
    let file: syn::File = syn::parse2(generated.clone()).expect("expected file-like tokens");

    let expected_str = expected.to_token_stream().to_string();

    let has_match = file.items.iter().any(|item| {
        let item = item.to_token_stream().to_string();
        item.contains(&expected_str)
    });

    assert!(
        has_match,
        "{}. Expected: {} In: {}",
        message,
        expected_str,
        generated.to_string()
    );
}
