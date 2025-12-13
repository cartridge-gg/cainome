use crate::expand::{types::CairoToRust, utils, Expandable, ExpansionContext, ExpansionResult};
use cainome_parser::tokens::{NamedToken, Struct, Token};
use proc_macro2::TokenStream;
use quote::quote;

pub fn struct_declaration(
    type_name: &str,
    fields: &Vec<NamedToken>,
    ctx: &ExpansionContext,
) -> TokenStream {
    let _ = ctx;

    let struct_name = utils::str_to_ident(&type_name);

    let mut members: Vec<TokenStream> = vec![];
    for inner in fields {
        let name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();

        let ty = utils::str_to_type(&token.to_rust_type());
        let serde = utils::serde_hex_derive(&token.to_rust_type());

        // r#{name} is not a valid identifier, thus we can't create an ident.
        // And with proc macro 2, we cannot do `quote!(r##name)`.
        // TODO: this needs to be done more elegantly...
        if &inner.name == "type" {
            members.push(quote!(#serde pub r#type: #ty));
        } else if &inner.name == "move" {
            members.push(quote!(#serde pub r#move: #ty));
        } else if &inner.name == "final" {
            members.push(quote!(#serde pub r#final: #ty));
        } else {
            members.push(quote!(#serde pub #name: #ty));
        }
    }

    let mut internal_derives = vec![];

    for d in ctx.derives.iter() {
        internal_derives.push(utils::str_to_type(d));
    }

    let derive = if internal_derives.len() > 0 {
        quote! { #[derive(#(#internal_derives,)*)] }
    } else {
        quote! {}
    };

    quote! {
        #derive
        pub struct #struct_name {
            #(#members),*
        }
    }
}

pub fn struct_implementation(
    type_name: &str,
    fields: &Vec<NamedToken>,
    _ctx: &ExpansionContext,
) -> TokenStream {
    let struct_name = utils::str_to_ident(&type_name);

    let mut sizes: Vec<TokenStream> = vec![];
    let mut sers: Vec<TokenStream> = vec![];
    let mut desers: Vec<TokenStream> = vec![];
    let mut names: Vec<TokenStream> = vec![];

    for inner in fields {
        let name = utils::str_to_ident(&inner.name);
        let token = &*inner.token.borrow();
        let ty = utils::str_to_type(&token.to_rust_type_path());

        // Tuples type used as rust type path item path must be surrounded
        // by angle brackets.
        let ty_punctuated = match &*inner.token.borrow() {
            Token::Tuple(_) => quote!(<#ty>),
            _ => quote!(#ty),
        };

        // r#{name} is not a valid identifier, thus we can't create an ident.
        // And with proc macro 2, we cannot do `quote!(r##name)`.
        // TODO: this needs to be done more elegantly...
        if &inner.name == "type" {
            names.push(quote!(r#type));

            sizes.push(quote! {
                __size += #ty_punctuated::cairo_serialized_size(&__rust.r#type);
            });

            sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.r#type));));

            desers.push(quote! {
                let r#type = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                __offset += #ty_punctuated::cairo_serialized_size(&r#type);
            });
        } else if &inner.name == "move" {
            names.push(quote!(r#move));

            sizes.push(quote! {
                __size += #ty_punctuated::cairo_serialized_size(&__rust.r#move);
            });

            sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.r#move));));

            desers.push(quote! {
                let r#move = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                __offset += #ty_punctuated::cairo_serialized_size(&r#move);
            });
        } else if &inner.name == "final" {
            names.push(quote!(r#final));

            sizes.push(quote! {
                __size += #ty_punctuated::cairo_serialized_size(&__rust.r#final);
            });

            sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.r#final));));

            desers.push(quote! {
                let r#final = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                __offset += #ty_punctuated::cairo_serialized_size(&r#final);
            });
        } else {
            names.push(quote!(#name));

            sizes.push(quote! {
                __size += #ty_punctuated::cairo_serialized_size(&__rust.#name);
            });

            sers.push(quote!(__out.extend(#ty_punctuated::cairo_serialize(&__rust.#name));));

            desers.push(quote! {
                let #name = #ty_punctuated::cairo_deserialize(__felts, __offset)?;
                __offset += #ty_punctuated::cairo_serialized_size(&#name);
            });
        }
    }

    let ccs = utils::cainome_cairo_serde();
    let snrs_types = utils::snrs_types();

    let (impl_line, rust_type) = (
        quote!(impl #ccs::CairoSerde for #struct_name),
        quote!(
            type RustType = Self;
        ),
    );

    quote! {
        #impl_line {

            #rust_type

            const SERIALIZED_SIZE: std::option::Option<usize> = None;

            #[inline]
            fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
                let mut __size = 0;
                #(#sizes)*
                __size
            }

            fn cairo_serialize(__rust: &Self::RustType) -> Vec<#snrs_types::Felt> {
                let mut __out: Vec<#snrs_types::Felt> = vec![];
                #(#sers)*
                __out
            }

            fn cairo_deserialize(__felts: &[#snrs_types::Felt], __offset: usize) -> #ccs::Result<Self::RustType> {
                let mut __offset = __offset;
                #(#desers)*
                Ok(#struct_name {
                    #(#names),*
                })
            }
        }
    }
}

impl Expandable for Struct {
    fn expand(&self, expansion_context: &ExpansionContext) -> Vec<ExpansionResult> {
        let module = self.type_module();
        let name = self.type_name();

        let declaration = struct_declaration(&name, &self.fields, expansion_context);
        let implementation = struct_implementation(&name, &self.fields, expansion_context);

        let item = quote! {
            #declaration

            #implementation
        };

        vec![ExpansionResult::new(&module).with_item(&name, item)]
    }
}

#[cfg(test)]
mod tests {
    use cainome_parser::{
        tokens::{NamedToken, Struct, Token},
        TypeRegistry,
    };
    use proc_macro2::TokenStream;
    use quote::ToTokens;
    use syn::{parse_quote, ItemStruct};

    use crate::expand::{Expandable, ExpansionContext, Module};

    fn assert_code_has<T: ToTokens>(generated: &TokenStream, expected: &T, message: &str) {
        let file: syn::File = syn::parse2(generated.clone()).expect("expected file-like tokens");

        let expected_str = expected.to_token_stream().to_string();

        let has_match = file.items.iter().any(|item| {
            let item = item.to_token_stream().to_string();
            item.contains(&expected_str)
        });

        assert!(
            has_match,
            "{}. Expected: {} In: {}",
            message,
            expected_str,
            generated.to_string()
        );
    }

    #[test]
    fn test_structure_expand_empty() {
        let registry = TypeRegistry::new();

        let structure = Struct::new("my::Type".to_string(), &registry).unwrap();

        let ctx = ExpansionContext::new("ContractName");

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        let expected: ItemStruct = parse_quote! {
            pub struct Type {}
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_structure_expand_basic_field() {
        let registry = TypeRegistry::new();

        let mut structure = Struct::new("my::Type".to_string(), &registry).unwrap();

        structure.fields.push(NamedToken {
            name: "f1".to_string(),
            token: registry.get("felt").unwrap(),
        });

        let ctx = ExpansionContext::new("ContractName");

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        let expected: ItemStruct = parse_quote! {
            pub struct Type {
                pub f1: starknet::core::types::Felt
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_structure_expand_with_derive() {
        let registry = TypeRegistry::new();

        let mut structure = Struct::new("my::Type".to_string(), &registry).unwrap();

        structure.fields.push(NamedToken {
            name: "f1".to_string(),
            token: registry.get("felt").unwrap(),
        });

        let ctx = ExpansionContext::new("ContractName").with_derives(vec!["Serde", "Clone"]);

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        let expected: ItemStruct = parse_quote! {
            #[derive(Serde, Clone, )]
            pub struct Type {
                pub f1: starknet::core::types::Felt
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_structure_expand_with_option_field() {
        let registry = TypeRegistry::new();

        let mut structure = Struct::new("my::Type".to_string(), &registry).unwrap();

        structure.fields.push(NamedToken {
            name: "f1".to_string(),
            token: registry.get("core::option::Option<felt>").unwrap(),
        });

        let ctx = ExpansionContext::new("ContractName").with_derives(vec!["Serde", "Clone"]);

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        let expected: ItemStruct = parse_quote! {
            #[derive(Serde, Clone, )]
            pub struct Type {
                pub f1: Option<starknet::core::types::Felt>
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_structure_expand_with_array_field() {
        let registry = TypeRegistry::new();

        let mut structure = Struct::new("my::Type".to_string(), &registry).unwrap();

        structure.fields.push(NamedToken {
            name: "f1".to_string(),
            token: registry.get("core::array::Array::<core::felt252>").unwrap(),
        });

        let ctx = ExpansionContext::new("ContractName").with_derives(vec!["Serde", "Clone"]);

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        let expected: ItemStruct = parse_quote! {
            #[derive(Serde, Clone, )]
            pub struct Type {
                pub f1: Vec<starknet::core::types::Felt>
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_structure_expand_with_non_zero_field() {
        let registry = TypeRegistry::new();

        let mut structure = Struct::new("my::Type".to_string(), &registry).unwrap();

        structure.fields.push(NamedToken {
            name: "f1".to_string(),
            token: registry
                .get("core::zeroable::NonZero::<core::felt252>")
                .unwrap(),
        });

        let ctx = ExpansionContext::new("ContractName").with_derives(vec!["Serde", "Clone"]);

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        let expected: ItemStruct = parse_quote! {
            #[derive(Serde, Clone, )]
            pub struct Type {
                pub f1: cainome::cairo_serde::NonZero<starknet::core::types::Felt>
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_structure_expand_with_tuple_field() {
        let registry = TypeRegistry::new();

        let mut structure = Struct::new("my::Type".to_string(), &registry).unwrap();

        structure.fields.push(NamedToken {
            name: "f1".to_string(),
            token: registry
                .get("(core::felt252, core::option::Option<felt>)")
                .unwrap(),
        });

        let ctx = ExpansionContext::new("ContractName").with_derives(vec!["Serde", "Clone"]);

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        println!("{}", generated.to_string());

        let expected: ItemStruct = parse_quote! {
            #[derive(Serde, Clone, )]
            pub struct Type {
                pub f1: (starknet::core::types::Felt, Option<starknet::core::types::Felt>)
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }

    #[test]
    fn test_structure_expand_with_self_reference() {
        let structure = {
            let mut registry = TypeRegistry::new();

            // Prepare placeholder
            registry.set("my::Type", Token::Placeholder);
            // Construct type
            let mut structure = Struct::new("my::Type".to_string(), &registry).unwrap();
            structure.fields.push(NamedToken {
                name: "f1".to_string(),
                token: registry.get("my::Type").unwrap(),
            });
            // Update placeholder
            registry.set("my::Type", Token::Struct(structure.clone()));

            structure
        };

        let ctx = ExpansionContext::new("ContractName").with_derives(vec!["Serde", "Clone"]);

        let generated = Module::new()
            .with_registered_many(structure.expand(&ctx))
            .to_token_stream();

        // TODO(@baitcode): This is incorrect. Should be Box<> or something.
        // Discuss with @glihm

        let expected: ItemStruct = parse_quote! {
            #[derive(Serde, Clone, )]
            pub struct Type {
                pub f1: Type
            }
        };

        assert_code_has(&generated, &expected, "Struct not found");
    }
}
