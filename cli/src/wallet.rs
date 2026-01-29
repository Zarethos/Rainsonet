//! Wallet management

use rainsonet_core::{Address, Amount, Nonce, RainsonetError, RainsonetResult, Timestamp};
use rainsonet_crypto::keys::KeyPair;
use rainsonet_relyo::RelyoTransaction;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Wallet file format
#[derive(Serialize, Deserialize)]
pub struct WalletFile {
    pub version: u32,
    pub name: String,
    pub address: String,
    pub public_key: String,
    pub encrypted_secret: Option<Vec<u8>>,
    pub plaintext_secret: Option<String>,
    pub created_at: u64,
}

/// Local wallet
pub struct Wallet {
    name: String,
    keypair: KeyPair,
    path: Option<PathBuf>,
}

impl Wallet {
    /// Create a new wallet
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            keypair: KeyPair::generate(),
            path: None,
        }
    }
    
    /// Create wallet from keypair
    pub fn from_keypair(name: &str, keypair: KeyPair) -> Self {
        Self {
            name: name.to_string(),
            keypair,
            path: None,
        }
    }
    
    /// Load wallet from file
    pub fn load(path: &PathBuf) -> RainsonetResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RainsonetError::Io(e.to_string()))?;
        
        let wallet_file: WalletFile = serde_json::from_str(&content)
            .map_err(|e| RainsonetError::Serialization(e.to_string()))?;
        
        // For now, only support plaintext (in production, implement encryption)
        let secret_hex = wallet_file.plaintext_secret
            .ok_or_else(|| RainsonetError::Config("No secret key in wallet".into()))?;
        
        let secret_bytes = hex::decode(&secret_hex)
            .map_err(|e| RainsonetError::Serialization(e.to_string()))?;
        
        let keypair = KeyPair::from_secret_bytes(&secret_bytes)?;
        
        Ok(Self {
            name: wallet_file.name,
            keypair,
            path: Some(path.clone()),
        })
    }
    
    /// Save wallet to file
    pub fn save(&self, path: &PathBuf) -> RainsonetResult<()> {
        let wallet_file = WalletFile {
            version: 1,
            name: self.name.clone(),
            address: self.address().to_hex(),
            public_key: self.keypair.public_key().to_hex(),
            encrypted_secret: None,
            plaintext_secret: Some(hex::encode(self.keypair.secret_bytes())),
            created_at: Timestamp::now().0,
        };
        
        let content = serde_json::to_string_pretty(&wallet_file)
            .map_err(|e| RainsonetError::Serialization(e.to_string()))?;
        
        std::fs::write(path, content)
            .map_err(|e| RainsonetError::Io(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get address
    pub fn address(&self) -> Address {
        self.keypair.address()
    }
    
    /// Get keypair
    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }
    
    /// Create and sign a transaction
    pub fn create_transaction(
        &self,
        to: Address,
        amount: Amount,
        fee: Amount,
        nonce: Nonce,
    ) -> RainsonetResult<RelyoTransaction> {
        RelyoTransaction::new(
            self.address(),
            to,
            amount,
            fee,
            nonce,
            &self.keypair,
        )
    }
}

/// Wallet manager for multiple wallets
pub struct WalletManager {
    wallets_dir: PathBuf,
}

impl WalletManager {
    pub fn new(wallets_dir: PathBuf) -> Self {
        Self { wallets_dir }
    }
    
    /// Create wallets directory if it doesn't exist
    pub fn init(&self) -> RainsonetResult<()> {
        std::fs::create_dir_all(&self.wallets_dir)
            .map_err(|e| RainsonetError::Io(e.to_string()))?;
        Ok(())
    }
    
    /// List all wallets
    pub fn list(&self) -> RainsonetResult<Vec<WalletInfo>> {
        self.init()?;
        
        let mut wallets = Vec::new();
        
        for entry in std::fs::read_dir(&self.wallets_dir)
            .map_err(|e| RainsonetError::Io(e.to_string()))?
        {
            let entry = entry.map_err(|e| RainsonetError::Io(e.to_string()))?;
            let path = entry.path();
            
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(wallet) = Wallet::load(&path) {
                    wallets.push(WalletInfo {
                        name: wallet.name().to_string(),
                        address: wallet.address().to_hex(),
                        path,
                    });
                }
            }
        }
        
        Ok(wallets)
    }
    
    /// Create a new wallet
    pub fn create(&self, name: &str) -> RainsonetResult<Wallet> {
        self.init()?;
        
        let wallet = Wallet::new(name);
        let path = self.wallets_dir.join(format!("{}.json", name));
        
        if path.exists() {
            return Err(RainsonetError::Config(format!(
                "Wallet '{}' already exists",
                name
            )));
        }
        
        wallet.save(&path)?;
        Ok(wallet)
    }
    
    /// Get wallet by name
    pub fn get(&self, name: &str) -> RainsonetResult<Wallet> {
        let path = self.wallets_dir.join(format!("{}.json", name));
        
        if !path.exists() {
            return Err(RainsonetError::Config(format!(
                "Wallet '{}' not found",
                name
            )));
        }
        
        Wallet::load(&path)
    }
    
    /// Import wallet from secret key
    pub fn import(&self, name: &str, secret_hex: &str) -> RainsonetResult<Wallet> {
        self.init()?;
        
        let secret_bytes = hex::decode(secret_hex)
            .map_err(|e| RainsonetError::Serialization(e.to_string()))?;
        
        let keypair = KeyPair::from_secret_bytes(&secret_bytes)?;
        let wallet = Wallet::from_keypair(name, keypair);
        
        let path = self.wallets_dir.join(format!("{}.json", name));
        wallet.save(&path)?;
        
        Ok(wallet)
    }
}

/// Wallet info for listing
#[derive(Debug)]
pub struct WalletInfo {
    pub name: String,
    pub address: String,
    pub path: PathBuf,
}
