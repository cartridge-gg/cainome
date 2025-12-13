mod test;

#[cfg(test)]
mod tests {
    use cainome_parser::AbiParser;

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
