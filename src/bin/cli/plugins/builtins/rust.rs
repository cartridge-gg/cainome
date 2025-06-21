use async_trait::async_trait;
use cainome_rs::{self};
use convert_case::{Case, Casing};

use crate::args::RustPluginOptions;
use crate::error::CainomeCliResult;
use crate::plugins::builtins::BuiltinPlugin;
use crate::plugins::PluginInput;

pub struct RustPlugin {
    options: RustPluginOptions,
}

impl RustPlugin {
    pub fn new(options: RustPluginOptions) -> Self {
        Self { options }
    }
}

#[async_trait]
impl BuiltinPlugin for RustPlugin {
    async fn generate_code(&self, input: &PluginInput) -> CainomeCliResult<()> {
        tracing::trace!("Rust plugin requested");

        for contract in &input.contracts {
            // The contract name contains the fully qualified path of the cairo module.
            // For now, let's only take the latest part of this path.
            // TODO: if a project has several contracts with the same name under different
            // namespaces, we should provide a solution to solve those conflicts.
            let contract_name = contract
                .name
                .split("::")
                .last()
                .unwrap_or(&contract.name)
                .from_case(Case::Snake)
                .to_case(Case::Pascal);

            let derives = self.options.derives.as_deref().unwrap_or_default();
            let contract_derives = self.options.contract_derives.as_deref().unwrap_or_default();

            let expanded = cainome_rs::abi_to_tokenstream(
                &contract_name,
                &contract.tokens,
                input.execution_version,
                derives,
                contract_derives,
                &input.type_skips,
            );
            let filename = format!(
                "{}.rs",
                contract_name.from_case(Case::Pascal).to_case(Case::Snake)
            );

            let mut out_path = input.output_dir.clone();
            out_path.push(filename);

            tracing::trace!("Rust writing file {}", out_path);
            std::fs::write(&out_path, expanded.to_string())?;
        }

        Ok(())
    }
}
