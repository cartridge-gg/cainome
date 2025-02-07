# Cainome Rust backend

This crates contains the run-time library to generate rust bindings (using Cairo Serde).

This crate is used as built-in plugin of cainome CLI, and is mainly published to expose the library used by `abigen!` macro in `cainome-rs-macro` crate.

For more details on what's generated, check the [rs-macro README](../rs-macro/README.md).

This crate however exposes a `Abigen` struct that can be used to programmatically generate bindings for a contract.

# Example

```rust
use cainome::rs::Abigen;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let mut aliases = HashMap::new();
    aliases.insert(String::from("my::type::Event"), String::from("MyTypeEvent"));

    let abigen = Abigen::new(
        "MyContract",
        "./contracts/target/dev/contracts_simple_get_set.contract_class.json",
    )
    .with_types_aliases(aliases)
    .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
    .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()]);

    abigen
        .generate()
        .expect("Fail to generate bindings")
        .write_to_file("/tmp/abigen.rs")
        .unwrap();
}
```
