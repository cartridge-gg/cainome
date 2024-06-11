// To run this example:
//
// cargo run --example typeshare --all-features
// Then you can run typeshare command on the generated file.
use cainome::rs::Abigen;

#[tokio::main]
async fn main() {
    let abigen =
        Abigen::new("MyContract", "./contracts/abi/simple_get_set.abi.json").with_typeshare();

    abigen
        .generate()
        .unwrap()
        .write_to_file("/tmp/with_typeshare.rs")
        .unwrap();
}
