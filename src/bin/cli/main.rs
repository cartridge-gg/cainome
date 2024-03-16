use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

mod args;
mod contract;
mod error;
mod plugins;

use args::CainomeArgs;
use contract::{ContractParser, ContractParserConfig};
use error::{CainomeCliResult, Error};
use plugins::{PluginInput, PluginManager};

#[tokio::main]
async fn main() -> CainomeCliResult<()> {
    init_logging()?;

    let args = CainomeArgs::parse();
    tracing::trace!("args: {:?}", args);

    let parser_config = if let Some(path) = args.parser_config {
        ContractParserConfig::from_json(&path)?
    } else {
        ContractParserConfig::default()
    };

    let contracts = if let Some(path) = args.artifacts_path {
        let ret = ContractParser::from_artifacts_path(path.clone(), &parser_config)?;

        if ret.is_empty() {
            tracing::error!(
                "No contract found with extension '{}' into '{}' directory",
                parser_config.sierra_extension,
                path
            );

            return Err(Error::Other("Invalid arguments".to_string()));
        }

        ret
    } else if let (Some(name), Some(address), Some(url)) =
        (args.contract_name, args.contract_address, args.rpc_url)
    {
        vec![ContractParser::from_chain(&name, address, url, &parser_config.type_aliases).await?]
    } else {
        panic!("Invalid arguments: no contracts to be parsed");
    };

    let pm = PluginManager::from(args.plugins);

    pm.generate(PluginInput {
        output_dir: args.output_dir,
        contracts,
    })
    .await?;

    Ok(())
}

pub fn init_logging() -> CainomeCliResult<()> {
    const DEFAULT_LOG_FILTER: &str = "info,cainome=trace";

    let builder = fmt::Subscriber::builder().with_env_filter(
        EnvFilter::try_from_default_env()
            .or(EnvFilter::try_new(DEFAULT_LOG_FILTER))
            .map_err(|e| Error::Other(format!("Tracing error: {:?}", e)))?,
    );

    tracing::subscriber::set_global_default(builder.finish())
        .map_err(|e| Error::Other(format!("Tracing error: {:?}", e)))
}
