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
