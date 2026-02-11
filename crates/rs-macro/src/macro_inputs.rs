//! Defines the arguments of the `abigen` macro.
//!
//! `ContractAbi` is expected to be the argument
//! passed to the macro. We should then parse the
//! token stream to ensure the arguments are correct.
//!
//! The macro supports two forms:
//!
//! 1. Loading from a file with the ABI array:
//!    abigen!(ContractName, "path/to/abi.json")
//!
//! 2. Direct JSON array input:
//!    abigen!(ContractName, [{"type": "function", ...}])
//!
use cainome_rs::expand::generic_resolver::{
    DefaultGenericResolver, GenericResolver, GenericResolverFromMapping,
};
use proc_macro_error::emit_error;
use quote::ToTokens;
use starknet::core::types::contract::{AbiEntry, SierraClass};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use std::str::FromStr;
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream, Result},
    Ident, LitStr, Token, Type,
};

use crate::macro_inputs_legacy::GenericMapping;
use crate::spanned::Spanned;
use cainome_rs::ExecutionVersion;

const CARGO_MANIFEST_DIR: &str = "$CARGO_MANIFEST_DIR/";

pub(crate) struct ContractAbi {
    pub name: Ident,
    pub abi: Vec<AbiEntry>,
    pub output_path: Option<String>,
    pub type_aliases: HashMap<String, String>,
    pub type_substitutions: HashMap<String, String>,
    pub execution_version: ExecutionVersion,
    pub derives: Vec<String>,
    pub contract_derives: Vec<String>,
    pub type_skips: Vec<String>,
    pub contract_source_path: Option<String>,
    pub add_declaration: bool,
    pub add_deployment: bool,
    pub cainome_serde_path: String,
    pub root_module_path: String,
    pub generic_resolver: Box<dyn GenericResolver>,
}

impl Parse for ContractAbi {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;

        // ABI path or content.

        // Parse either a JSON array literal or a path to the contract class artifact.
        //
        // If the input starts with a `[` token then we parse it as a JSON array.
        let abi = if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let array_content: proc_macro2::TokenStream = content.parse()?;

            let array_str = format!("[{array_content}]");

            serde_json::from_str::<Vec<AbiEntry>>(&array_str)
                .map_err(|e| syn::Error::new(input.span(), format!("Invalid ABI format: {e}")))?
        }
        // Otherwise, parse it either as a path to the full JSON contract class artifact
        else {
            // Handle file path case
            let abi_str_or_path = input.parse::<LitStr>()?;

            if abi_str_or_path.value().ends_with(".json") {
                let json_path = if abi_str_or_path.value().starts_with(CARGO_MANIFEST_DIR) {
                    let manifest_dir = env!("CARGO_MANIFEST_DIR");
                    let new_dir = Path::new(manifest_dir)
                        .join(
                            abi_str_or_path
                                .value()
                                .trim_start_matches(CARGO_MANIFEST_DIR),
                        )
                        .to_string_lossy()
                        .to_string();

                    LitStr::new(&new_dir, proc_macro2::Span::call_site())
                } else {
                    abi_str_or_path
                };

                let mut file = open_json_file(&json_path.value())?;

                // To prepare the declare and deploy features, we also
                // accept a full Sierra artifact for the ABI.
                // To support declare and deploy, the full class must be stored.
                if let Ok(class) = serde_json::from_reader::<_, SierraClass>(BufReader::new(&file))
                {
                    class.abi
                } else {
                    // Reset the file pointer back to the beginning of the file.
                    let pos = SeekFrom::Start(0);
                    file.seek(pos).expect("failed to reset file pointer");

                    serde_json::from_reader::<_, Vec<AbiEntry>>(BufReader::new(&file)).map_err(
                        |e| syn::Error::new(json_path.span(), format!("JSON parse error: {e}")),
                    )?
                }
            } else {
                serde_json::from_str::<Vec<AbiEntry>>(&abi_str_or_path.value()).map_err(|e| {
                    syn::Error::new(abi_str_or_path.span(), format!("JSON parse error: {e}"))
                })?
            }
        };

        let mut output_path: Option<String> = None;
        let mut execution_version = ExecutionVersion::V3;
        let mut type_aliases = HashMap::new();
        let mut type_substitutions = HashMap::new();
        let mut derives = Vec::new();
        let mut contract_derives = Vec::new();
        let mut type_skips = Vec::new();
        let mut contract_source_path = None;
        let mut add_declaration = true;
        let mut add_deployment = true;
        let mut cainome_serde_path = "cainome::cairo_serde".to_string();
        let mut root_module_path = "self".to_string();
        let mut generic_resolver: Box<dyn GenericResolver> = Box::new(DefaultGenericResolver);

        loop {
            if input.parse::<Token![,]>().is_err() {
                break;
            }

            let name = match Ident::parse_any(input) {
                Ok(n) => n,
                Err(_) => break,
            };

            match name.to_string().as_str() {
                "generic_resolver" => {
                    let content;
                    braced!(content in input);
                    let parsed =
                        content.parse_terminated(Spanned::<GenericMapping>::parse, Token![;])?;

                    let mappings = parsed
                        .into_iter()
                        .map(|p| (p.r#type.clone(), p.field.clone(), p.generic_arg.clone()))
                        .collect::<Vec<_>>();

                    generic_resolver = Box::new(GenericResolverFromMapping::new(mappings));
                }
                "type_aliases" => {
                    let content;
                    braced!(content in input);
                    let parsed =
                        content.parse_terminated(Spanned::<TypeAlias>::parse, Token![;])?;

                    let mut abi_types = HashSet::new();
                    let mut aliases = HashSet::new();

                    for type_alias in parsed {
                        if !abi_types.insert(type_alias.abi.clone()) {
                            emit_error!(
                                type_alias.span(),
                                format!("{} duplicate abi type", type_alias.abi)
                            );
                        }
                        if !aliases.insert(type_alias.alias.clone()) {
                            emit_error!(
                                type_alias.span(),
                                format!("{} duplicate alias name", type_alias.alias)
                            );
                        }

                        let ta = type_alias.into_inner();
                        type_aliases.insert(ta.abi, ta.alias);
                    }
                }
                "type_substitutions" => {
                    let content;
                    braced!(content in input);
                    let parsed =
                        content.parse_terminated(Spanned::<TypeSubstitution>::parse, Token![;])?;

                    let mut abi_types = HashSet::new();
                    let mut aliases = HashSet::new();

                    for type_sub in parsed {
                        if !abi_types.insert(type_sub.abi.clone()) {
                            emit_error!(
                                type_sub.span(),
                                format!("{} duplicate abi type", type_sub.abi)
                            );
                        }
                        if !aliases.insert(type_sub.sub.clone()) {
                            emit_error!(
                                type_sub.span(),
                                format!("{} duplicate substitution name", type_sub.sub)
                            );
                        }

                        let ta = type_sub.into_inner();
                        type_substitutions.insert(ta.abi, ta.sub);
                    }
                }
                "output_path" => {
                    let content;
                    parenthesized!(content in input);
                    output_path = Some(content.parse::<LitStr>()?.value());
                }
                "execution_version" => {
                    let content;
                    parenthesized!(content in input);
                    let ev = content.parse::<LitStr>()?.value();
                    execution_version = ExecutionVersion::from_str(&ev).map_err(|e| {
                        syn::Error::new(content.span(), format!("Invalid execution version: {e}"))
                    })?;
                }
                "derives" => {
                    let content;
                    parenthesized!(content in input);
                    let parsed = content.parse_terminated(Spanned::<Type>::parse, Token![,])?;

                    for derive in parsed {
                        derives.push(derive.to_token_stream().to_string());
                    }
                }
                "contract_derives" => {
                    let content;
                    parenthesized!(content in input);
                    let parsed = content.parse_terminated(Spanned::<Type>::parse, Token![,])?;

                    for derive in parsed {
                        contract_derives.push(derive.to_token_stream().to_string());
                    }
                }
                "type_skips" => {
                    let content;
                    parenthesized!(content in input);
                    let parsed = content.parse_terminated(Spanned::<Type>::parse, Token![,])?;

                    for type_skip in parsed {
                        type_skips.push(type_skip.to_token_stream().to_string());
                    }
                }
                "contract_source_path" => {
                    let content;
                    parenthesized!(content in input);
                    contract_source_path = Some(content.parse::<LitStr>()?.value());
                }
                "add_declaration" => {
                    let content;
                    parenthesized!(content in input);
                    add_declaration = content.parse::<syn::LitBool>()?.value();
                }
                "add_deployment" => {
                    let content;
                    parenthesized!(content in input);
                    add_deployment = content.parse::<syn::LitBool>()?.value();
                }
                "cainome_serde_path" => {
                    let content;
                    parenthesized!(content in input);
                    cainome_serde_path = content.parse::<LitStr>()?.value();
                }
                "root_module_path" => {
                    let content;
                    parenthesized!(content in input);
                    root_module_path = content.parse::<LitStr>()?.value();
                }
                _ => emit_error!(name.span(), format!("unexpected named parameter `{name}`")),
            }
        }

        Ok(ContractAbi {
            name,
            abi,
            output_path,
            type_aliases,
            type_substitutions,
            execution_version,
            derives,
            contract_derives,
            type_skips,
            contract_source_path,
            add_declaration,
            add_deployment,
            cainome_serde_path,
            root_module_path,
            generic_resolver,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypeSubstitution {
    abi: String,
    sub: String,
}

impl Parse for TypeSubstitution {
    fn parse(input: ParseStream) -> Result<Self> {
        let abi = sanitize_str(&input.parse::<Type>()?.into_token_stream().to_string());

        input.parse::<Token![as]>()?;

        let sub = sanitize_str(&input.parse::<Type>()?.into_token_stream().to_string());

        Ok(TypeSubstitution { abi, sub })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypeAlias {
    abi: String,
    alias: String,
}

impl Parse for TypeAlias {
    fn parse(input: ParseStream) -> Result<Self> {
        let abi = sanitize_str(&input.parse::<Type>()?.into_token_stream().to_string());

        input.parse::<Token![as]>()?;

        let alias = sanitize_str(&input.parse::<Ident>()?.to_string());

        Ok(TypeAlias { abi, alias })
    }
}

fn sanitize_str(abi: &str) -> String {
    abi.trim().replace([' ', '\n', '\t'], "").to_string()
}

fn open_json_file(file_path: &str) -> Result<File> {
    File::open(file_path).map_err(|e| {
        syn::Error::new(
            str_to_litstr(file_path).span(),
            format!("JSON open file {file_path} error: {e}"),
        )
    })
}

pub fn str_to_litstr(str_in: &str) -> LitStr {
    LitStr::new(str_in, proc_macro::Span::call_site().into())
}
