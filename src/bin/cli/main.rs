use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

mod args;
mod contract;
mod error;
mod plugins;

use args::CainomeArgs;
use contract::ContractParser;
use plugins::{PluginInput, PluginManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging()?;

    let config = CainomeArgs::parse();
    tracing::trace!("config: {:?}", config);

    let contracts = if let Some(path) = config.artifacts_path {
        ContractParser::from_artifacts_path(path)?
    } else if let (Some(name), Some(address), Some(url)) = (
        config.contract_name,
        config.contract_address,
        config.rpc_url,
    ) {
        vec![ContractParser::from_chain(&name, address, url).await?]
    } else {
        panic!("Invalid arguments: no contracts to be parsed");
    };

    let pm = PluginManager::from(config.plugins);

    pm.generate(PluginInput {
        output_dir: config.output_dir,
        contracts,
    })
    .await?;

    Ok(())
}

pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    const DEFAULT_LOG_FILTER: &str = "info,cainome=trace";

    let builder = fmt::Subscriber::builder().with_env_filter(
        EnvFilter::try_from_default_env().or(EnvFilter::try_new(DEFAULT_LOG_FILTER))?,
    );

    Ok(tracing::subscriber::set_global_default(builder.finish())?)
}
