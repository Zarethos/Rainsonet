//! RAINSONET CLI - Command Line Interface

use clap::{Parser, Subcommand};
use rainsonet_cli::{
    build_transaction_request, ApiClient, Wallet, WalletManager,
};
use rainsonet_core::Amount;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "relyo")]
#[command(about = "RELYO - RAINSONET Payment CLI")]
#[command(version)]
struct Cli {
    /// Node URL
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    node: String,
    
    /// Wallets directory
    #[arg(short, long, default_value = "./wallets")]
    wallets_dir: PathBuf,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Wallet operations
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },
    
    /// Get account balance
    Balance {
        /// Address to check (or wallet name with --wallet)
        address: Option<String>,
        
        /// Use wallet by name
        #[arg(short, long)]
        wallet: Option<String>,
    },
    
    /// Send RELYO tokens
    Send {
        /// Sender wallet name
        #[arg(short, long)]
        from: String,
        
        /// Recipient address
        #[arg(short, long)]
        to: String,
        
        /// Amount to send (in RELYO units)
        #[arg(short, long)]
        amount: f64,
        
        /// Transaction fee (in RELYO units)
        #[arg(long, default_value = "0.001")]
        fee: f64,
        
        /// Nonce (optional, auto-fetch if not provided)
        #[arg(long)]
        nonce: Option<u64>,
    },
    
    /// Get transaction status
    Transaction {
        /// Transaction ID
        tx_id: String,
    },
    
    /// Node status
    Status,
}

#[derive(Subcommand)]
enum WalletAction {
    /// Create a new wallet
    Create {
        /// Wallet name
        name: String,
    },
    
    /// List all wallets
    List,
    
    /// Show wallet info
    Info {
        /// Wallet name
        name: String,
    },
    
    /// Import wallet from secret key
    Import {
        /// Wallet name
        name: String,
        
        /// Secret key (hex)
        secret: String,
    },
    
    /// Export wallet secret key
    Export {
        /// Wallet name
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let wallet_manager = WalletManager::new(cli.wallets_dir);
    let api_client = ApiClient::new(&cli.node);
    
    match cli.command {
        Commands::Wallet { action } => {
            handle_wallet_command(action, &wallet_manager)?;
        }
        
        Commands::Balance { address, wallet } => {
            let addr = if let Some(wallet_name) = wallet {
                let w = wallet_manager.get(&wallet_name)?;
                w.address().to_hex()
            } else if let Some(a) = address {
                a
            } else {
                eprintln!("Error: Provide either an address or --wallet");
                std::process::exit(1);
            };
            
            match api_client.get_balance(&addr).await {
                Ok(info) => {
                    println!("Address:  {}", info.address);
                    println!("Balance:  {} RELYO", info.balance_relyo);
                    println!("(Raw:     {} wei)", info.balance);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Send { from, to, amount, fee, nonce } => {
            let wallet = wallet_manager.get(&from)?;
            
            // Get nonce if not provided
            let tx_nonce = match nonce {
                Some(n) => n,
                None => {
                    let account = api_client.get_account(&wallet.address().to_hex()).await?;
                    account.nonce
                }
            };
            
            // Convert amounts
            let amount_wei = Amount::from_relyo_f64(amount);
            let fee_wei = Amount::from_relyo_f64(fee);
            
            // Build and send transaction
            let tx_req = build_transaction_request(&wallet, &to, amount_wei, fee_wei, tx_nonce)?;
            
            println!("Sending {} RELYO to {}...", amount, to);
            
            match api_client.submit_transaction(&tx_req).await {
                Ok(resp) => {
                    println!("✅ Transaction submitted!");
                    println!("TX ID:  {}", resp.tx_id);
                    println!("Status: {}", resp.status);
                }
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Transaction { tx_id } => {
            match api_client.get_transaction(&tx_id).await {
                Ok(resp) => {
                    println!("TX ID:  {}", resp.tx_id);
                    println!("Status: {}", resp.status);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Status => {
            match api_client.status().await {
                Ok(status) => {
                    println!("🌧️ RAINSONET Node Status");
                    println!("========================");
                    println!("Node ID:       {}", truncate(&status.node_id, 16));
                    println!("State Version: {}", status.state_version);
                    println!("State Root:    {}", truncate(&status.state_root, 16));
                    println!("Peer Count:    {}", status.peer_count);
                    println!("Is Validator:  {}", if status.is_validator { "Yes" } else { "No" });
                    println!("Mempool Size:  {}", status.mempool_size);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    
    Ok(())
}

fn handle_wallet_command(action: WalletAction, manager: &WalletManager) -> anyhow::Result<()> {
    match action {
        WalletAction::Create { name } => {
            let wallet = manager.create(&name)?;
            println!("✅ Wallet '{}' created!", name);
            println!("Address: {}", wallet.address().to_hex());
        }
        
        WalletAction::List => {
            let wallets = manager.list()?;
            
            if wallets.is_empty() {
                println!("No wallets found.");
            } else {
                println!("Wallets:");
                println!("{:<20} {}", "Name", "Address");
                println!("{:-<20} {:-<66}", "", "");
                for w in wallets {
                    println!("{:<20} {}", w.name, w.address);
                }
            }
        }
        
        WalletAction::Info { name } => {
            let wallet = manager.get(&name)?;
            println!("Wallet: {}", wallet.name());
            println!("Address: {}", wallet.address().to_hex());
            println!("Public Key: {}", wallet.keypair().public_key().to_hex());
        }
        
        WalletAction::Import { name, secret } => {
            let wallet = manager.import(&name, &secret)?;
            println!("✅ Wallet '{}' imported!", name);
            println!("Address: {}", wallet.address().to_hex());
        }
        
        WalletAction::Export { name } => {
            let wallet = manager.get(&name)?;
            println!("⚠️  Keep this secret key safe!");
            println!("Secret Key: {}", hex::encode(wallet.keypair().secret_bytes()));
        }
    }
    
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
