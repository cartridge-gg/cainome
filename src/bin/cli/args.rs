//! Cainome CLI arguments.
//!
use cainome_rs::ExecutionVersion;
use camino::Utf8PathBuf;
use clap::{Args, Parser};
use starknet::core::types::Felt;
use url::Url;

use crate::plugins::PluginManager;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct CainomeArgs {
    #[arg(long)]
    #[arg(value_name = "OUTPUT_DIR")]
    #[arg(help = "Directory where bindings files must be written.")]
    pub output_dir: Utf8PathBuf,

    #[arg(long)]
    #[arg(value_name = "PATH")]
    #[arg(conflicts_with = "contract_address")]
    #[arg(
        help = "Path where artifacts are located. Cainome will parse all the files that are a valid Sierra artifact."
    )]
    pub artifacts_path: Option<Utf8PathBuf>,

    #[arg(long)]
    #[arg(value_name = "PATH")]
    #[arg(help = "Path of a JSON file defining Cainome parsing configuration.")]
    pub parser_config: Option<Utf8PathBuf>,

    #[arg(long)]
    #[arg(value_name = "ADDRESS")]
    #[arg(conflicts_with = "artifacts_path")]
    #[arg(requires = "rpc_url")]
    #[arg(requires = "contract_name")]
    #[arg(help = "Address of the contract to fetch the ABI from.")]
    pub contract_address: Option<Felt>,

    #[arg(long)]
    #[arg(value_name = "NAME")]
    #[arg(requires = "contract_address")]
    #[arg(requires = "rpc_url")]
    #[arg(help = "Name of the contract.")]
    pub contract_name: Option<String>,

    #[arg(long)]
    #[arg(value_name = "URL")]
    #[arg(requires = "contract_address")]
    #[arg(requires = "contract_name")]
    #[arg(conflicts_with = "artifacts_path")]
    #[arg(help = "The Starknet RPC provider to fetch the ABI from.")]
    pub rpc_url: Option<Url>,

    #[arg(long)]
    #[arg(value_name = "EXECUTION_VERSION")]
    #[arg(help = "The execution version to use. Supported values are 'v1', 'V1', 'v3', or 'V3'.")]
    pub execution_version: ExecutionVersion,

    #[arg(long)]
    #[arg(value_name = "type_skips")]
    #[arg(help = "Types to be skipped from the generated types.")]
    pub type_skips: Option<Vec<String>>,

    #[command(flatten)]
    #[command(next_help_heading = "Plugins options")]
    pub plugins: PluginOptions,
}

#[derive(Debug, Args, Clone)]
pub struct PluginOptions {
    #[arg(long)]
    #[arg(help = "Generate bindings for rust (built-in).")]
    pub rust: bool,

    #[command(flatten)]
    #[command(next_help_heading = "Rust plugin options")]
    pub rust_options: RustPluginOptions,

    #[arg(long)]
    #[arg(help = "Generate bindings for golang (built-in).")]
    pub golang: bool,

    #[command(flatten)]
    #[command(next_help_heading = "Golang plugin options")]
    pub golang_options: GolangPluginOptions,
    // TODO: For custom plugin, we can add a vector of strings,
    // where the user provides the name of the plugin.
    // Then cainome like protobuf will attempt to execute cainome_plugin_<NAME>.
}

#[derive(Debug, Args, Clone)]
pub struct RustPluginOptions {
    #[arg(long = "rust-derives")]
    #[arg(value_name = "DERIVES")]
    #[arg(help = "Derives to be added to the generated types (Rust plugin).")]
    pub derives: Option<Vec<String>>,

    #[arg(long = "rust-contract-derives")]
    #[arg(value_name = "CONTRACT_DERIVES")]
    #[arg(help = "Derives to be added to the generated contract (Rust plugin).")]
    pub contract_derives: Option<Vec<String>>,
}

#[derive(Debug, Args, Clone)]
pub struct GolangPluginOptions {
    #[arg(long = "golang-package")]
    #[arg(value_name = "PACKAGE_NAME")]
    #[arg(default_value = "abigen")]
    #[arg(help = "Go package name for generated bindings (Golang plugin).")]
    pub package_name: String,
}

impl From<PluginOptions> for PluginManager {
    fn from(options: PluginOptions) -> Self {
        let mut builtin_plugins: Vec<Box<dyn crate::plugins::builtins::BuiltinPlugin>> = vec![];
        // Ignored for now.
        let plugins = vec![];

        if options.rust {
            builtin_plugins.push(Box::new(crate::plugins::builtins::RustPlugin::new(
                options.rust_options,
            )));
        }

        if options.golang {
            builtin_plugins.push(Box::new(crate::plugins::builtins::GolangPlugin::new(
                options.golang_options,
            )));
        }

        Self {
            builtin_plugins,
            plugins,
        }
    }
}
