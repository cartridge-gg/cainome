cargo test --workspace --all-features

# Somes examples are currently containing some generated
# code to test the serde implementation.
# TODO: this should be moved to the test suite.
cargo run --example structs --all-features
