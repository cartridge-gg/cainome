use cainome_rs::ExecutionVersion;
use camino::Utf8PathBuf;

pub mod builtins;
use builtins::BuiltinPlugins;

use crate::contract::ContractData;
use crate::error::CainomeCliResult;
use crate::plugins::builtins::{BuiltinPlugin, RustPlugin};

#[derive(Debug)]
pub struct PluginInput {
    pub output_dir: Utf8PathBuf,
    pub contracts: Vec<ContractData>,
    pub execution_version: ExecutionVersion,
    pub derives: Vec<String>,
}

#[derive(Debug)]
pub struct PluginManager {
    /// A list of builtin plugins to invoke as rust module.
    pub builtin_plugins: Vec<BuiltinPlugins>,
    /// A list of custom plugins to invoke via stdin.
    pub plugins: Vec<String>,
}

impl PluginManager {
    /// Generates the bindings by calling all the configured Plugin.
    pub async fn generate(&self, input: PluginInput) -> CainomeCliResult<()> {
        if self.builtin_plugins.is_empty() && self.plugins.is_empty() {
            return Ok(());
        }

        for bp in &self.builtin_plugins {
            let builder: Box<dyn BuiltinPlugin> = match bp {
                BuiltinPlugins::Rust => Box::new(RustPlugin::new()),
            };

            builder.generate_code(&input).await?;
        }

        // TODO: add the plugins once stdin is supported.
        // To ensure that -> use JSON to send the list of contracts + the output dir
        // to the plugin via stdin.
        // + define a plugin output to know if it was a success of not + the list
        // of generated files.

        Ok(())
    }
}

// TODO: stdin interface to allow development of plugins
// in other languages.
