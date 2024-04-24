//! Cainome CLI arguments.
//!
use camino::Utf8PathBuf;
use clap::{Args, Parser};
use starknet::core::types::FieldElement;
use url::Url;

use crate::plugins::builtins::BuiltinPlugins;
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
    pub contract_address: Option<FieldElement>,

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

    #[command(flatten)]
    #[command(next_help_heading = "Plugins options")]
    pub plugins: PluginOptions,
}

#[derive(Debug, Args, Clone)]
pub struct PluginOptions {
    #[arg(long)]
    #[arg(help = "Generate bindings for rust (built-in).")]
    pub rust: bool,
    // TODO: For custom plugin, we can add a vector of strings,
    // where the user provides the name of the plugin.
    // Then cainome like protobuf will attempt to execute cainome_plugin_<NAME>.
}

impl From<PluginOptions> for PluginManager {
    fn from(options: PluginOptions) -> Self {
        let mut builtin_plugins = vec![];
        // Ignored for now.
        let plugins = vec![];

        if options.rust {
            builtin_plugins.push(BuiltinPlugins::Rust);
        }

        Self {
            builtin_plugins,
            plugins,
        }
    }
}
