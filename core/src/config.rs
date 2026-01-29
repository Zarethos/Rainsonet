//! Configuration types for RAINSONET

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node name for logging
    pub name: String,
    
    /// Data directory
    pub data_dir: PathBuf,
    
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    
    /// API configuration
    pub api: ApiConfig,
    
    /// Logging level
    pub log_level: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            name: "rainsonet-node".to_string(),
            data_dir: PathBuf::from("./data"),
            network: NetworkConfig::default(),
            consensus: ConsensusConfig::default(),
            api: ApiConfig::default(),
            log_level: "info".to_string(),
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen address for P2P
    pub listen_addr: String,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    
    /// Maximum peer connections
    pub max_peers: usize,
    
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    
    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/0.0.0.0/tcp/30333".to_string(),
            bootstrap_nodes: vec![],
            max_peers: 50,
            enable_mdns: true,
            connection_timeout: 30,
        }
    }
}

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// This node is a validator
    pub is_validator: bool,
    
    /// Minimum validators for consensus
    pub min_validators: usize,
    
    /// Required vote percentage (0-100)
    pub vote_threshold: u8,
    
    /// Proposal timeout in milliseconds
    pub proposal_timeout_ms: u64,
    
    /// Vote timeout in milliseconds
    pub vote_timeout_ms: u64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            is_validator: false,
            min_validators: 3,
            vote_threshold: 67, // 2/3 majority
            proposal_timeout_ms: 5000,
            vote_timeout_ms: 3000,
        }
    }
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Enable HTTP API
    pub enabled: bool,
    
    /// API listen address
    pub listen_addr: String,
    
    /// Enable CORS
    pub enable_cors: bool,
    
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            listen_addr: "127.0.0.1:8080".to_string(),
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// RELYO module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelyoConfig {
    /// Minimum transaction fee
    pub min_fee: u128,
    
    /// Fee burn percentage (0-100)
    pub fee_burn_percent: u8,
    
    /// Maximum transaction amount per tx
    pub max_tx_amount: u128,
    
    /// Transaction expiry time in seconds
    pub tx_expiry_seconds: u64,
    
    /// Initial supply (for genesis)
    pub initial_supply: u128,
}

impl Default for RelyoConfig {
    fn default() -> Self {
        Self {
            min_fee: 1_000_000_000_000_000, // 0.001 RELYO
            fee_burn_percent: 50,
            max_tx_amount: 1_000_000_000_000_000_000_000_000, // 1M RELYO
            tx_expiry_seconds: 3600, // 1 hour
            initial_supply: 100_000_000_000_000_000_000_000_000, // 100M RELYO
        }
    }
}
