use async_trait::async_trait;
use cainome_rs::{self};

use crate::error::CainomeCliResult;
use crate::plugins::builtins::BuiltinPlugin;
use crate::plugins::PluginInput;

pub struct RustPlugin;

impl RustPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl BuiltinPlugin for RustPlugin {
    async fn generate_code(&self, input: &PluginInput) -> CainomeCliResult<()> {
        tracing::trace!("Rust plugin requested:\n{:?}\n", input);

        for contract in &input.contracts {
            let expanded = cainome_rs::abi_to_tokenstream(&contract.name, &contract.tokens);
            let filename = format!("{}.rs", contract.name);

            let mut out_path = input.output_dir.clone();
            out_path.push(filename);

            tracing::trace!("Rust writing file {}", out_path);
            std::fs::write(&out_path, expanded.to_string())?;
        }

        Ok(())
    }
}
