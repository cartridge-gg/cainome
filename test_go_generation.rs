use std::collections::HashMap;
use std::fs;
use camino::Utf8PathBuf;

// Import the modules from the binary
use std::path::Path;

// We need to use the modules from the binary
use cainome_rs::ExecutionVersion;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Go generation...");
    
    // Create test output directory
    let test_output_dir = Utf8PathBuf::from("./test_output");
    fs::create_dir_all(&test_output_dir)?;
    
    // Parse contracts from the ABI directory
    let config = ContractParserConfig {
        sierra_extension: ".abi.json".to_string(),
        type_aliases: HashMap::new(),
    };
    
    let parser = ContractParser::new(config);
    let contracts = parser.parse_contracts_from_directory("contracts/abi")?;
    
    println!("Found {} contracts", contracts.len());
    
    if contracts.is_empty() {
        println!("No contracts found, creating a simple test contract...");
        return Ok(());
    }
    
    // Generate Go bindings
    let golang_plugin = plugins::builtins::golang::GolangPlugin::new(GolangPluginOptions {
        package_name: "abigen".to_string(),
    });
    
    for contract in &contracts {
        println!("Generating Go bindings for: {}", contract.name);
        
        let plugin_input = PluginInput {
            output_dir: test_output_dir.clone(),
            contracts: vec![contract.clone()],
            execution_version: cainome_rs::ExecutionVersion::V3,
            type_skips: vec![],
        };
        
        match golang_plugin.generate_code(&plugin_input).await {
            Ok(_) => println!("Successfully generated Go bindings for {}", contract.name),
            Err(e) => println!("Failed to generate Go bindings for {}: {:?}", contract.name, e),
        }
    }
    
    // Check generated files
    let generated_files: Vec<_> = fs::read_dir(&test_output_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("go") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    
    println!("Generated {} Go files", generated_files.len());
    
    for file_path in &generated_files {
        println!("Generated: {}", file_path.display());
    }
    
    Ok(())
}