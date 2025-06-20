use std::fmt;

use cainome_rs::ExecutionVersion;
use camino::Utf8PathBuf;

pub mod builtins;

use crate::contract::ContractData;
use crate::error::CainomeCliResult;
use crate::plugins::builtins::{BuiltinPlugin, GolangPlugin, RustPlugin};

#[derive(Debug)]
pub struct PluginInput {
    pub output_dir: Utf8PathBuf,
    pub contracts: Vec<ContractData>,
    pub execution_version: ExecutionVersion,
    pub type_skips: Vec<String>,
}

pub struct PluginManager {
    /// A list of builtin plugins to invoke as rust module.
    pub builtin_plugins: Vec<Box<dyn BuiltinPlugin>>,
    /// A list of custom plugins to invoke via stdin.
    pub plugins: Vec<String>,
}

impl PluginManager {
    /// Generates the bindings by calling all the configured Plugin.
    pub async fn generate(&self, input: PluginInput) -> CainomeCliResult<()> {
        if self.builtin_plugins.is_empty() && self.plugins.is_empty() {
            return Ok(());
        }

        for plugin in &self.builtin_plugins {
            plugin.generate_code(&input).await?;
        }

        // TODO: add the plugins once stdin is supported.
        // To ensure that -> use JSON to send the list of contracts + the output dir
        // to the plugin via stdin.
        // + define a plugin output to know if it was a success of not + the list
        // of generated files.

        Ok(())
    }
}

impl fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginManager")
            .field(
                "builtin_plugins",
                &format!("{} plugins", self.builtin_plugins.len()),
            )
            .field("plugins", &self.plugins)
            .finish()
    }
}

// TODO: stdin interface to allow development of plugins
// in other languages.
