//! RAINSONET Node Binary

use clap::{Parser, Subcommand};
use rainsonet_core::NodeConfig;
use rainsonet_crypto::keys::KeyPair;
use rainsonet_node::{NodeBuilder, RainsonetNode};
use rainsonet_relyo::GenesisConfig;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "rainsonet-node")]
#[command(about = "RAINSONET Node - Decentralized Payment Rail")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the node
    Run {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Genesis file path
        #[arg(short, long)]
        genesis: Option<PathBuf>,
        
        /// Run as validator
        #[arg(long)]
        validator: bool,
        
        /// API listen address
        #[arg(long, default_value = "127.0.0.1:8080")]
        api_addr: String,
        
        /// P2P listen address
        #[arg(long, default_value = "/ip4/0.0.0.0/tcp/30333")]
        p2p_addr: String,
        
        /// Data directory
        #[arg(long, default_value = "./data")]
        data_dir: PathBuf,
    },
    
    /// Generate a new keypair
    Keygen {
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Generate genesis configuration
    Genesis {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
        
        /// Chain name
        #[arg(long, default_value = "RAINSONET Devnet")]
        chain_name: String,
        
        /// Chain ID
        #[arg(long, default_value = "3")]
        chain_id: u64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .pretty()
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run {
            config,
            genesis,
            validator,
            api_addr,
            p2p_addr,
            data_dir,
        } => {
            info!("🌧️ Starting RAINSONET Node...");
            
            // Load or create keypair
            let keypair = load_or_create_keypair(&data_dir)?;
            
            // Load or create genesis
            let genesis_config = match genesis {
                Some(path) => {
                    let content = std::fs::read_to_string(&path)?;
                    GenesisConfig::from_json(&content)?
                }
                None => GenesisConfig::devnet(),
            };
            
            // Build node
            let mut builder = NodeBuilder::new()
                .keypair(keypair)
                .genesis(genesis_config.clone())
                .api_addr(&api_addr)
                .p2p_addr(&p2p_addr);
            
            if validator {
                builder = builder.validator();
            }
            
            let node = builder.build();
            
            // Start node
            node.start(genesis_config).await?;
        }
        
        Commands::Keygen { output } => {
            let keypair = KeyPair::generate();
            
            let info = serde_json::json!({
                "public_key": keypair.public_key().to_hex(),
                "address": keypair.address().to_hex(),
                "secret_key": hex::encode(keypair.secret_bytes()),
            });
            
            let json = serde_json::to_string_pretty(&info)?;
            
            match output {
                Some(path) => {
                    std::fs::write(&path, &json)?;
                    println!("Keypair saved to: {}", path.display());
                }
                None => {
                    println!("{}", json);
                }
            }
        }
        
        Commands::Genesis {
            output,
            chain_name,
            chain_id,
        } => {
            let genesis = GenesisConfig {
                chain_name,
                chain_id,
                ..GenesisConfig::devnet()
            };
            
            let json = genesis.to_json()?;
            std::fs::write(&output, &json)?;
            
            println!("Genesis configuration saved to: {}", output.display());
        }
    }
    
    Ok(())
}

fn load_or_create_keypair(data_dir: &PathBuf) -> anyhow::Result<KeyPair> {
    let key_path = data_dir.join("node_key.json");
    
    if key_path.exists() {
        let content = std::fs::read_to_string(&key_path)?;
        let value: serde_json::Value = serde_json::from_str(&content)?;
        
        if let Some(secret_hex) = value.get("secret_key").and_then(|v| v.as_str()) {
            let secret_bytes = hex::decode(secret_hex)?;
            let keypair = KeyPair::from_secret_bytes(&secret_bytes)?;
            info!("Loaded keypair from {}", key_path.display());
            return Ok(keypair);
        }
    }
    
    // Create new keypair
    std::fs::create_dir_all(data_dir)?;
    
    let keypair = KeyPair::generate();
    
    let info = serde_json::json!({
        "public_key": keypair.public_key().to_hex(),
        "address": keypair.address().to_hex(),
        "secret_key": hex::encode(keypair.secret_bytes()),
    });
    
    std::fs::write(&key_path, serde_json::to_string_pretty(&info)?)?;
    info!("Generated new keypair, saved to {}", key_path.display());
    
    Ok(keypair)
}
