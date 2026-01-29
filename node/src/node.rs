//! Full node implementation

use crate::api::start_api_server;
use crate::runtime::NodeRuntime;
use rainsonet_core::NodeConfig;
use rainsonet_crypto::keys::KeyPair;
use rainsonet_relyo::GenesisConfig;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

/// Full RAINSONET node
pub struct RainsonetNode {
    runtime: Arc<NodeRuntime>,
}

impl RainsonetNode {
    /// Create a new node
    pub fn new(config: NodeConfig, keypair: KeyPair, genesis: GenesisConfig) -> Self {
        let runtime = Arc::new(NodeRuntime::new(config, keypair, genesis));
        Self { runtime }
    }
    
    /// Start the node
    pub async fn start(&self, genesis: GenesisConfig) -> anyhow::Result<()> {
        info!("Starting RAINSONET node...");
        
        // Initialize genesis
        self.runtime.initialize_genesis(genesis).await?;
        
        // Start API server
        let api_runtime = self.runtime.clone();
        let api_addr = self.runtime.config().api.listen_addr.clone();
        
        let api_handle = tokio::spawn(async move {
            if let Err(e) = start_api_server(api_runtime, &api_addr).await {
                error!("API server error: {}", e);
            }
        });
        
        info!("Node started successfully");
        info!("Node ID: {}", self.runtime.node_id().map(|id| id.to_hex()).unwrap_or_default());
        info!("Is Validator: {}", self.runtime.is_validator());
        
        // Wait for shutdown signal
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Shutdown signal received, stopping node...");
            }
            Err(e) => {
                error!("Error waiting for shutdown signal: {}", e);
            }
        }
        
        // Cleanup
        api_handle.abort();
        
        info!("Node stopped");
        
        Ok(())
    }
    
    /// Get runtime reference
    pub fn runtime(&self) -> &Arc<NodeRuntime> {
        &self.runtime
    }
}

/// Node builder for easier configuration
pub struct NodeBuilder {
    config: NodeConfig,
    keypair: Option<KeyPair>,
    genesis: GenesisConfig,
}

impl NodeBuilder {
    pub fn new() -> Self {
        Self {
            config: NodeConfig::default(),
            keypair: None,
            genesis: GenesisConfig::devnet(),
        }
    }
    
    pub fn config(mut self, config: NodeConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn keypair(mut self, keypair: KeyPair) -> Self {
        self.keypair = Some(keypair);
        self
    }
    
    pub fn genesis(mut self, genesis: GenesisConfig) -> Self {
        self.genesis = genesis;
        self
    }
    
    pub fn validator(mut self) -> Self {
        self.config.consensus.is_validator = true;
        self
    }
    
    pub fn api_addr(mut self, addr: &str) -> Self {
        self.config.api.listen_addr = addr.to_string();
        self
    }
    
    pub fn p2p_addr(mut self, addr: &str) -> Self {
        self.config.network.listen_addr = addr.to_string();
        self
    }
    
    pub fn build(self) -> RainsonetNode {
        let keypair = self.keypair.unwrap_or_else(KeyPair::generate);
        RainsonetNode::new(self.config, keypair, self.genesis)
    }
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
