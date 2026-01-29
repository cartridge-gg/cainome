use crate::error::CainomeCliResult;
use crate::plugins::PluginInput;

mod rust;
pub use rust::RustPlugin;

// mod golang;
// pub use golang::GolangPlugin;

pub trait BuiltinPlugin {
    /// Generates code by executing the plugin.
    ///
    /// # Arguments
    ///
    /// * `data` - Contract data.
    fn generate_code(&self, input: &PluginInput) -> CainomeCliResult<()>;
}
