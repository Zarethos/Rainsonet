//! Genesis configuration for RELYO

use rainsonet_core::{Address, Amount, RainsonetError, RainsonetResult, RelyoConfig};
use rainsonet_state::StateStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::ledger::RelyoLedger;

/// Genesis allocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAllocation {
    pub address: String,
    pub balance: String,
}

/// Genesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Chain name
    pub chain_name: String,
    /// Chain ID
    pub chain_id: u64,
    /// Genesis timestamp
    pub timestamp: u64,
    /// Initial validator addresses
    pub validators: Vec<String>,
    /// Initial token allocations
    pub allocations: Vec<GenesisAllocation>,
    /// RELYO config
    #[serde(default)]
    pub relyo_config: RelyoConfig,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            chain_name: "RAINSONET Mainnet".to_string(),
            chain_id: 1,
            timestamp: 0,
            validators: vec![],
            allocations: vec![],
            relyo_config: RelyoConfig::default(),
        }
    }
}

impl GenesisConfig {
    /// Create a testnet genesis config
    pub fn testnet() -> Self {
        Self {
            chain_name: "RAINSONET Testnet".to_string(),
            chain_id: 2,
            ..Default::default()
        }
    }
    
    /// Create a devnet genesis config for development
    pub fn devnet() -> Self {
        Self {
            chain_name: "RAINSONET Devnet".to_string(),
            chain_id: 3,
            relyo_config: RelyoConfig {
                min_fee: 0, // Free for development
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    /// Add a validator
    pub fn add_validator(mut self, address: &str) -> Self {
        self.validators.push(address.to_string());
        self
    }
    
    /// Add an allocation
    pub fn add_allocation(mut self, address: &str, balance_relyo: u64) -> Self {
        self.allocations.push(GenesisAllocation {
            address: address.to_string(),
            balance: format!("{}", Amount::from_relyo(balance_relyo).0),
        });
        self
    }
    
    /// Parse allocations into address -> amount map
    pub fn parse_allocations(&self) -> RainsonetResult<HashMap<Address, Amount>> {
        let mut result = HashMap::new();
        
        for alloc in &self.allocations {
            let address = Address::from_hex(&alloc.address)
                .map_err(|e| RainsonetError::InvalidAddress(e.to_string()))?;
            
            let balance: u128 = alloc
                .balance
                .parse()
                .map_err(|e| RainsonetError::InvalidTransaction(format!("Invalid balance: {}", e)))?;
            
            result.insert(address, Amount::new(balance));
        }
        
        Ok(result)
    }
    
    /// Calculate total supply from allocations
    pub fn total_supply(&self) -> RainsonetResult<Amount> {
        let allocations = self.parse_allocations()?;
        let total: u128 = allocations.values().map(|a| a.0).sum();
        Ok(Amount::new(total))
    }
    
    /// Save to JSON file
    pub fn to_json(&self) -> RainsonetResult<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| RainsonetError::SerializationError(e.to_string()))
    }
    
    /// Load from JSON
    pub fn from_json(json: &str) -> RainsonetResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| RainsonetError::DeserializationError(e.to_string()))
    }
}

/// Initialize genesis state
pub struct GenesisInitializer<S: StateStore> {
    ledger: std::sync::Arc<RelyoLedger<S>>,
    config: GenesisConfig,
}

impl<S: StateStore + 'static> GenesisInitializer<S> {
    pub fn new(ledger: std::sync::Arc<RelyoLedger<S>>, config: GenesisConfig) -> Self {
        Self { ledger, config }
    }
    
    /// Initialize the genesis state
    pub async fn initialize(&self) -> RainsonetResult<()> {
        info!("Initializing genesis for chain: {}", self.config.chain_name);
        info!("Chain ID: {}", self.config.chain_id);
        
        let allocations = self.config.parse_allocations()?;
        
        for (address, balance) in allocations {
            self.ledger.set_balance(&address, balance).await?;
            info!("Genesis allocation: {} = {}", address, balance);
        }
        
        let total = self.config.total_supply()?;
        info!("Total genesis supply: {}", total);
        
        Ok(())
    }
}

/// Genesis state hash computation
pub fn compute_genesis_hash(config: &GenesisConfig) -> RainsonetResult<rainsonet_core::Hash> {
    let json = config.to_json()?;
    Ok(rainsonet_crypto::hashing::hash(json.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    use rainsonet_state::MemoryStateStore;
    use std::sync::Arc;
    
    #[test]
    fn test_genesis_config() {
        let kp = KeyPair::generate();
        let address = kp.address().to_hex();
        
        let config = GenesisConfig::devnet()
            .add_validator(&address)
            .add_allocation(&address, 1000000);
        
        assert_eq!(config.validators.len(), 1);
        assert_eq!(config.allocations.len(), 1);
    }
    
    #[test]
    fn test_genesis_json() {
        let config = GenesisConfig::testnet();
        let json = config.to_json().unwrap();
        let restored = GenesisConfig::from_json(&json).unwrap();
        
        assert_eq!(config.chain_name, restored.chain_name);
        assert_eq!(config.chain_id, restored.chain_id);
    }
    
    #[tokio::test]
    async fn test_genesis_initialization() {
        let kp = KeyPair::generate();
        let address = kp.address().to_hex();
        
        let config = GenesisConfig::devnet()
            .add_allocation(&address, 1000);
        
        let state = Arc::new(MemoryStateStore::new());
        let ledger = Arc::new(RelyoLedger::new(state, config.relyo_config.clone()));
        
        let initializer = GenesisInitializer::new(ledger.clone(), config);
        initializer.initialize().await.unwrap();
        
        let balance = ledger.get_balance(&kp.address()).await.unwrap();
        assert_eq!(balance.0, Amount::from_relyo(1000).0);
    }
}
