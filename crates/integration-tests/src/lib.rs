#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cainome_parser::AbiParser;
    use starknet::core::types::contract::SierraClass;

    #[test]
    fn test_something() {
        let abi = include_str!("../../../contracts/abi/basic.abi.json");
        let z = AbiParser::parse_abi_string(abi).unwrap();
        let type_aliases: HashMap<String, String> = HashMap::new();
        let x = AbiParser::collect_tokens(z, &type_aliases);
        assert!(true);
        // let tokens = AbiParser::tokens_from_abi_string(&abi, &HashMap::new()).unwrap();
        // assert_ne!(tokens.enums.len(), 0);
        // assert_ne!(tokens.functions.len(), 0);
        // assert_ne!(tokens.interfaces.len(), 0);
        // assert_ne!(tokens.structs.len(), 0);
    }
}
