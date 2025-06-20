use async_trait::async_trait;

use crate::error::CainomeCliResult;
use crate::plugins::PluginInput;

mod rust;
pub use rust::RustPlugin;

mod golang;
pub use golang::GolangPlugin;

#[async_trait]
pub trait BuiltinPlugin {
    /// Generates code by executing the plugin.
    ///
    /// # Arguments
    ///
    /// * `data` - Contract data.
    async fn generate_code(&self, input: &PluginInput) -> CainomeCliResult<()>;
}
