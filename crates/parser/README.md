# Parser (Cainome)

A run-time library to generate an intermediate representation of Cairo ABI.

The parser is in charge to parse the ABI entries and convert then into tokens. Those tokens represent the ABI in a comprehensive manner to then be used for lowering into different languages.

# Tokens

The Cairo ABI is represented by a set of 6 tokens:

- basic (`CoreBasic`): corelib types, which are every types starting with `core::` that can fit into a single felt and the unit (`()`) type. This excludes `Array`, which is processed on it's own token.
- array (`Array`): `Array` and `Span` are included in this token. `Span` is normally a struct, but considered as `Array` by the parser.
- tuple (`Tuple`): tuple of any length >= 1.
- composite (`Composite`): any type defined in the ABI as a struct or an enum. All composite type name is automatically converted into `PascalCase`.
- function (`Function`): views and externals functions.
- generic argument (`GenericArg`): a generic argument, resolved with it's letter (`A`, `B`...).

# Genericity

As Cairo is a language that support generic arguments, the ABI does not include any information about the generic argument typing. Types are totally flatten in the ABI.

```rust
struct GenericOne<A> {
    a: A,
    b: felt252,
    c: u256,
}
```

Will exist in the ABI as many time as a function uses `GenericOne` with a different `A` value.
For instance, if one function output is:

```
fn my_func(self: @ContractState) -> GenericOne<felt252>;
```

The ABI will contain:

```json
  {
    "type": "struct",
    "name": "contracts::abicov::structs::GenericOne::<core::felt252>",
    "members": [
      {
        "name": "a",
        "type": "core::felt252"
      },
      {
        "name": "b",
        "type": "core::felt252"
      },
      {
        "name": "c",
        "type": "core::integer::u256"
      }
    ]
  },
```

And here as you can see, we've lost the information about the genericity.

To deal with that, Cainome has (for now) a very simple algorithm:

1. It gathers all the structs and enums with the exact same type path `contracts::abicov::structs::GenericOne` in this example.
2. It resolves the genericity of `structs` and `enums`, meaning that if the generic argument is `core::felt252`, all the tokens found in the members (recursively) will have the `CoreBasic` token replaced by `GenericArg` and the corresponding letter. In the example above, the member `a` will become `GenericArg("A")`.
3. Finally, the tokens are ordered in a map with `structs`, `enums` and `functions`.

# Events

Events at top level are `enums`. And those enums, have some variants that are `struct` and others are `enums`. The parser clearly labels any composite that is an event, which allow further processing dedicated for the events.

Auto deserialization from `EmittedEvent` coming soon.
