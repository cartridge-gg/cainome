# Cainome Rust abigen

This crates contains the compile-time rust macro `abigen` to generate rust bindings (using Cairo Serde).

## Import in your project

```toml
# Cargo.toml

cainome = { version = "0.5.0", features = ["abigen-rs"] }
```

```rust
// Rust code
use cainome::rs::abigen;

abigen!(MyContract, "/path/my_contract.json");
```

Cairo 0 support is limited (event are not parsed yet), but to interact with a cairo 0
program you can use the legacy macro:

```rust
// Rust code
use cainome::rs::abigen;

abigen_legacy!(MyContract, "/path/cairo_0.json");
```

## Usage

For examples, please refer to the [examples](../../examples) folder.

The `abigen!` macro takes 2 or 3 inputs:

1. The name you want to assign to the contract type being generated.
2. Path to the JSON file containing the ABI. This file can have two format:

   - The entire Sierra file (`*.contract_class.json`) [**Only for Cairo 1**]
   - Only the array of ABI entries. These can be easily extracted with `jq` doing the following:

   ```
   jq .abi ./target/dev/package_contract.contract_class.json > /path/contract.json
   ```

3. Optional parameters:
   - `output_path`: if provided, the content will be generated in the given file instead of being expanded at the location of the macro invocation.
   - `type_aliases`: to avoid type name conflicts between components / contracts, you can rename some type by providing an alias for the full type path. It is important to give the **full** type path to ensure aliases are applied correctly.
   - `derive`: to specify the derive for the generated structs/enums.
   - `contract_derives`: to specify the derive for the generated contract type.

```rust
use cainome::rs::abigen;

// Default.
abigen!(MyContract, "/path/contract.json");

// Example with optional output path:
abigen!(MyContract, "/path/contract.json", output_path("/path/module.rs"));

// Example type aliases:
abigen!(
    MyContract,
    "./contracts/abi/components.abi.json",
    type_aliases {
        package::module1::component1::MyStruct as MyStruct1;
        package::module2::component2::MyStruct as MyStruct2;
    },
);

// Example with custom derives:
abigen!(
    MyContract,
    "./contracts/abi/components.abi.json",
    derives(Debug, Clone),
    contract_derives(Debug, Clone)
);

fn main() {
    // ... use the generated types here, which all of them
    // implement CairoSerde trait.
}
```

As a known limitation of `Cargo`, the `/path/contract.json` is relative to the Cargo manifest (`Cargo.toml`). This is important when executing a specific package (`-p`) or from the workspace (`--workspace/--all`), the manifest directory is not the same!

## What is generated

The expansion of the macros generates the following:

- For every type that is exposed in the ABI, a `struct` or `enum` will be generated with the `CairoSerde` trait automatically derived. The name of the type if always the last segment of the full type path, enforced to be in `PascalCase`.

  ```rust
  // Take this cairo struct, in with the full path `package::my_contract::MyStruct
  MyStruct {
    a: felt252,
    b: u256,
  }

  // This will generate a rust struct with the make `MyStruct`:
  MyStruct {
    a: starknet::core::types::Felt,
    a: U256, // Note the `PascalCase` here. As `u256` is a struct, it follows the common rule.
  }
  ```

- **Contract** type with the identifier of your choice (`MyContract` in the previous example). This type contains all the functions (externals and views) of your contract being exposed in the ABI. To initialize this type, you need the contract address and any type that implements `ConnectedAccount` from `starknet-rs`. Remember that `Arc<ConnectedAccount>` also implements `ConnectedAccount`.
  ```rust
  let account = SingleOwnerAccount::new(...);
  let contract_address = Felt::from_hex("0x1234...");
  let contract = MyContract::new(contract_address, account);
  ```
- **Contract Reader** type with the identifier of your choice with the suffix `Reader` (`MyContractReader`) in the previous example. The reader contains only the views of your contract. To initialize a reader, you need the contract address and a provider from `starknet-rs`.
  ```rust
  let provider = AnyProvider::JsonRpcHttp(...);
  let contract_address = Felt::from_hex("0x1234...");
  let contract_reader = MyContractReader::new(contract_address, &provider);
  ```
- For each **view**, the contract type and the contract reader type contain a function with the exact same arguments. Calling the function returns a `cainome_cairo_serde::call::FCall` struct to allow you to customize how you want the function to be called. Currently, the only setting is the `block_id`. Finally, to actually do the RPC call, you have to use `call()` method on the `FCall` struct.
  The default `block_id` value is `BlockTag::Pending`.
  ```rust
  let my_struct = contract
      .get_my_struct()
      .block_id(BlockId::Tag(BlockTag::Latest))
      .call()
      .await
      .expect("Call to `get_my_struct` failed");
  ```
- For each **external**, the contract type contains a function with the same arguments. Calling the function return a `starknet::accounts::ExecutionV1` type from `starknet-rs`, which allows you to completly customize the fees, doing only a simulation etc... To actually send the transaction, you use the `send()` method on the `ExecutionV3` struct. You can find the [associated methods with this struct on starknet-rs repo](https://github.com/xJonathanLEI/starknet-rs/blob/171b0c65cac407ee33972e0ab2c3f8744c083753/starknet-accounts/src/account/execution.rs#L403).

  ```rust
  let my_struct = MyStruct {
      a: Felt::ONE,
      b: U256 {
          low: 1,
          high: 0,
      }
  };

  let tx_res = contract
      .set_my_struct(&my_struct)
      .send()
      .await
      .expect("Call to `set_my_struct` failed");
  ```

  To support multicall, currently `ExecutionV1` type does not expose the `Call`s.
  To circumvey this, for each of the external function an other function with `_getcall` suffix is generated:

  ```rust
  // Gather the `Call`s.
  let set_a_call = contract.set_a_getcall(&Felt::ONE);
  let set_b_call = contract.set_b_getcall(&U256 { low: 0xff, high: 0 });

  // Then use the account exposed by the `MyContract` type to realize the multicall.
  let tx_res = contract
      .account
      .execute(vec![set_a_call, set_b_call])
      .send()
      .await
      .expect("Multicall failed");
  ```

- For each `Event` enumeration in the contract, the trait `TryFrom<EmittedEvent>` is generated. `EmittedEvent` is the type used
  by `starknet-rs` when events are fetched using `provider.get_events()`.

  ```rust
  let events = provider.get_events(...).await.unwrap();

  for event in events {
  match event.try_into() {
    Ok(ev) => {
        // Here, `ev` is deserialized + selectors are checked.
    }
    Err(e) => {
        trace!("Event can't be deserialized to any known Event variant: {e}");
        continue;
    }
  };
  ```

- For cairo 0 contracts, for each method that has at least one output, cainome will generate a `struct` with the output fields.

  ```json
  {
    "inputs": [],
    "name": "get_blockhash_registry",
    "outputs": [
      {
        "name": "address",
        "type": "felt"
      }
    ],
    "stateMutability": "view",
    "type": "function"
  }
  ```

  Will generate with the function's name in PascalCase and the suffix `Output`:

  ```rust
  pub struct GetBlockhashRegistryOutput {
      pub address: starknet::core::types::Felt,
  }
  ```

## Known limitation

With the current state of the parser, here are some limitations:

1. Generic arguments: even if the library currently supports generic arguments, sometimes the simple algorithm for generic resolution is not able to re-construct the expected generic mapping. This may cause compilation errors. Take an example with:

```rust
struct GenericTwo<A, B> {
    a: A,
    b: B,
    c: felt252,
}
```

If the cairo code only have one use of this struct like this:

```rust
fn my_func(self: @ContractState) -> GenericTwo<u64, u64>;
```

Then the ABI will look like this:

```json
  {
    "type": "struct",
    "name": "contracts::abicov::structs::GenericTwo::<core::integer::u64, core::integer::u64>",
    "members": [
      {
        "name": "a",
        "type": "core::integer::u64"
      },
      {
        "name": "b",
        "type": "core::integer::u64"
      },
      {
        "name": "c",
        "type": "core::felt252"
      }
    ]
  },
```

And here... how can we know that `a` is `A` and `b` is `B`? The current algorithm will generate the following:

```rust
struct GenericTwo<A, B> {
    a: A,
    b: A,
    c: felt252,
}
```

Which will cause a compilation error.

A first approach to this, is to add a `Phantom` placeholder for each of the variant. To ensure that there is always the two generic args used. But this will prevent the easy initialization of the struct with the fields. Need to check if we can use `Default`, or instead, using a `new(..)` pattern.

## Roadmap

1. [ ] Add a simple transaction status watcher integrated to the contract type.
2. [ ] Add declare and deploy function to the contract type.
3. [ ] Custom choice of derive for generated structs/enums.
