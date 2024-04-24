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
cd contracts/
make generate_artifacts
```

3. Setup (declare and deploy) the contract of your choice:

```
make setup_simple_get_set
```

4. Run the example

```
cargo run --example simple_get_set --all-features
```
