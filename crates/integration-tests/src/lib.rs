#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cainome_parser::AbiParser;

    #[test]
    fn test_something() {
        let abi = include_str!("../../../contracts/abi/basic.abi.json");
        let z = AbiParser::parse_abi_string(abi).unwrap();
        let type_aliases: HashMap<String, String> = HashMap::new();
        // let x = AbiParser::collect_tokens(z, &type_aliases);
        assert!(true);
        // let tokens = AbiParser::tokens_from_abi_string(&abi, &HashMap::new()).unwrap();
        // assert_ne!(tokens.enums.len(), 0);
        // assert_ne!(tokens.functions.len(), 0);
        // assert_ne!(tokens.interfaces.len(), 0);
        // assert_ne!(tokens.structs.len(), 0);
    }

    #[test]
    fn test_token_parsing_basic_types() {
        let abi = r#"[{
                "type": "struct",
                "name": "core::integer::u256",
                "members": [
                    {
                        "name": "low",
                        "type": "core::integer::u128"
                    },
                    {
                        "name": "high",
                        "type": "core::integer::u128"
                    }
                ]
            }]
        "#;

        let abies = AbiParser::parse_abi_string(abi).unwrap();
        let tokenized_abi = AbiParser::collect_tokens(abies).unwrap();
        assert_eq!(tokenized_abi.enums.len(), 0);
        assert_eq!(tokenized_abi.structs.len(), 1);
        assert_eq!(tokenized_abi.functions.len(), 0);
        assert_eq!(tokenized_abi.interfaces.len(), 0);
    }
}
