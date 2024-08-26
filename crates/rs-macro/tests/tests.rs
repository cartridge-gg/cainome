#[test]
fn test_compile_fail_abigen() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/abigen/*.rs");
}
