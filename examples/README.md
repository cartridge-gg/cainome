# Examples

## Usage

To run the example, please consider the following:

1. Spin up a katana instance ([install here](curl -L https://install.dojoengine.org | bash)):

```
dojoup -v
katana
```

2. Generates the artifacts to have the abi being extracted:

```
make generate_artifacts
```

3. Setup (declare and deploy) the contract of your choice:

```
make setup_simple_get_set
```

4. Run the example

```
cargo run --example simple_get_set --features="abigen-rs"
```

## IMPORTANT

Currently Starkli does not support `2.4.0` compiler. The examples are compiled using `2.4.0` to test all the features
including the latest, but if you experience errors while deploying with starkli, consider re-compiling with `2.3.1` (except for `byte_array`, which is only supported by `2.4.0`).
