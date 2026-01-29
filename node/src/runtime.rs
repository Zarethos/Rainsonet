//! Node runtime combining all components

use rainsonet_consensus::{
    ConsensusEvent, LocalValidator, RainsonetConsensus, SharedValidatorSet, ValidatorInfo,
    ValidatorSet,
};
use rainsonet_core::{
    Address, Amount, Hash, NodeConfig, NodeId, Nonce, RainsonetResult, StateChange,
    StateRoot, StateVersion,
};
use rainsonet_crypto::keys::KeyPair;
use rainsonet_p2p::{create_network_channel, NetworkEvent, NetworkService};
use rainsonet_relyo::{
    create_mempool, Account, GenesisConfig, GenesisInitializer, RelyoLedger, SharedMempool,
    VerifiedTransaction,
};
use rainsonet_state::{create_memory_store, MemoryStateStore, SharedMemoryStateStore};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Node runtime managing all components
pub struct NodeRuntime {
    config: NodeConfig,
    keypair: KeyPair,
    state: SharedMemoryStateStore,
    ledger: Arc<RelyoLedger<MemoryStateStore>>,
    mempool: SharedMempool,
    consensus: Arc<RainsonetConsensus>,
    validator_set: SharedValidatorSet,
    state_version: parking_lot::RwLock<StateVersion>,
    state_root: parking_lot::RwLock<StateRoot>,
}

impl NodeRuntime {
    /// Create a new node runtime
    pub fn new(config: NodeConfig, keypair: KeyPair, genesis: GenesisConfig) -> Self {
        // Initialize state store
        let state = create_memory_store();
        
        // Initialize RELYO ledger
        let ledger = Arc::new(RelyoLedger::new(state.clone(), genesis.relyo_config.clone()));
        
        // Initialize mempool
        let mempool = create_mempool(10000, 100);
        
        // Initialize validator set
        let validator_set = Arc::new(ValidatorSet::new());
        
        // Add self as validator if configured
        if config.consensus.is_validator {
            let validator_info = ValidatorInfo::new(
                keypair.node_id(),
                keypair.public_key(),
                1000, // Default stake
            );
            validator_set.add_validator(validator_info);
        }
        
        // Initialize consensus engine
        let consensus_keypair = if config.consensus.is_validator {
            Some(keypair.clone())
        } else {
            None
        };
        
        let consensus = Arc::new(RainsonetConsensus::new(
            config.consensus.clone(),
            validator_set.clone(),
            consensus_keypair,
        ));
        
        Self {
            config,
            keypair,
            state,
            ledger,
            mempool,
            consensus,
            validator_set,
            state_version: parking_lot::RwLock::new(StateVersion::new(0)),
            state_root: parking_lot::RwLock::new(Hash::ZERO),
        }
    }
    
    /// Initialize genesis state
    pub async fn initialize_genesis(&self, genesis: GenesisConfig) -> RainsonetResult<()> {
        let initializer = GenesisInitializer::new(self.ledger.clone(), genesis);
        initializer.initialize().await?;
        
        // Compute initial state root
        let root = self.state.compute_root().await?;
        *self.state_root.write() = root;
        
        info!("Genesis initialized, state root: {}", root);
        
        Ok(())
    }
    
    /// Get node ID
    pub fn node_id(&self) -> Option<NodeId> {
        Some(self.keypair.node_id())
    }
    
    /// Check if validator
    pub fn is_validator(&self) -> bool {
        self.config.consensus.is_validator
    }
    
    /// Get current state version
    pub fn state_version(&self) -> StateVersion {
        *self.state_version.read()
    }
    
    /// Get current state root
    pub fn state_root(&self) -> StateRoot {
        *self.state_root.read()
    }
    
    /// Get peer count (placeholder)
    pub fn peer_count(&self) -> usize {
        0 // TODO: Implement when network is connected
    }
    
    /// Get mempool size
    pub fn mempool_size(&self) -> usize {
        self.mempool.size()
    }
    
    /// Get mempool transaction IDs
    pub fn mempool_tx_ids(&self) -> Vec<Hash> {
        self.mempool.all_tx_ids()
    }
    
    /// Check if transaction is pending
    pub fn is_transaction_pending(&self, tx_id: &Hash) -> bool {
        self.mempool.contains(tx_id)
    }
    
    /// Get account
    pub async fn get_account(&self, address: &Address) -> RainsonetResult<Account> {
        self.ledger.get_account(address).await
    }
    
    /// Get balance
    pub async fn get_balance(&self, address: &Address) -> RainsonetResult<Amount> {
        self.ledger.get_balance(address).await
    }
    
    /// Get nonce
    pub async fn get_nonce(&self, address: &Address) -> RainsonetResult<Nonce> {
        self.ledger.get_nonce(address).await
    }
    
    /// Submit a transaction
    pub async fn submit_transaction(&self, tx: VerifiedTransaction) -> RainsonetResult<Hash> {
        let tx_id = tx.tx_id;
        
        // Validate against current state
        let validator = rainsonet_relyo::RelyoTransactionValidator::new(
            self.ledger.config().clone(),
        );
        validator.validate(&tx.tx, &*self.state).await?;
        
        // Add to mempool
        if !self.mempool.add(tx)? {
            return Err(rainsonet_core::RainsonetError::InvalidTransaction(
                "Failed to add to mempool".into(),
            ));
        }
        
        info!("Transaction {} added to mempool", tx_id);
        
        // If validator, try to propose block
        if self.is_validator() {
            self.try_propose_block().await?;
        }
        
        Ok(tx_id)
    }
    
    /// Try to propose a block with pending transactions
    async fn try_propose_block(&self) -> RainsonetResult<()> {
        // Get executable transactions
        let transactions = self.mempool.get_executable(100);
        
        if transactions.is_empty() {
            return Ok(());
        }
        
        info!("Proposing block with {} transactions", transactions.len());
        
        // Execute transactions and collect changes
        let mut all_changes = Vec::new();
        let mut tx_ids = Vec::new();
        
        for verified in transactions {
            match self.ledger.execute_transaction(&verified).await {
                Ok(changes) => {
                    all_changes.extend(changes);
                    tx_ids.push(verified.tx_id);
                }
                Err(e) => {
                    warn!("Transaction {} failed: {}", verified.tx_id, e);
                    self.mempool.remove(&verified.tx_id);
                }
            }
        }
        
        if all_changes.is_empty() {
            return Ok(());
        }
        
        // Compute new state root
        let previous_root = *self.state_root.read();
        let new_root = rainsonet_crypto::hashing::hash(&bincode::serialize(&all_changes)?);
        
        // Create proposal
        let proposal = self.consensus.create_proposal(
            previous_root,
            new_root,
            tx_ids.clone(),
            all_changes.clone(),
        )?;
        
        // For single node or when consensus is reached immediately
        // (In production, this would wait for votes from other validators)
        
        // Apply changes
        let new_version = self.state.apply_batch(all_changes).await?;
        self.ledger.commit().await?;
        
        // Update state
        *self.state_version.write() = new_version;
        *self.state_root.write() = new_root;
        
        // Remove from mempool
        for tx_id in tx_ids {
            self.mempool.remove(&tx_id);
        }
        
        info!(
            "Block finalized: version={}, root={}, tx_count={}",
            new_version, new_root, proposal.tx_ids.len()
        );
        
        Ok(())
    }
    
    /// Get keypair reference
    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }
    
    /// Get config reference
    pub fn config(&self) -> &NodeConfig {
        &self.config
    }
    
    /// Get ledger reference
    pub fn ledger(&self) -> &Arc<RelyoLedger<MemoryStateStore>> {
        &self.ledger
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_core::Amount;
    
    fn create_test_runtime() -> NodeRuntime {
        let config = NodeConfig {
            consensus: rainsonet_core::ConsensusConfig {
                is_validator: true,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let keypair = KeyPair::generate();
        let genesis = GenesisConfig::devnet();
        
        NodeRuntime::new(config, keypair, genesis)
    }
    
    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = create_test_runtime();
        
        assert!(runtime.is_validator());
        assert_eq!(runtime.state_version().0, 0);
    }
    
    #[tokio::test]
    async fn test_genesis_initialization() {
        let runtime = create_test_runtime();
        let kp = KeyPair::generate();
        
        let genesis = GenesisConfig::devnet()
            .add_allocation(&kp.address().to_hex(), 1000);
        
        runtime.initialize_genesis(genesis).await.unwrap();
        
        let balance = runtime.get_balance(&kp.address()).await.unwrap();
        assert_eq!(balance.0, Amount::from_relyo(1000).0);
    }
    
    #[tokio::test]
    async fn test_transaction_submission() {
        let runtime = create_test_runtime();
        let sender = KeyPair::generate();
        let recipient = KeyPair::generate();
        
        // Initialize with balance
        let genesis = GenesisConfig::devnet()
            .add_allocation(&sender.address().to_hex(), 1000);
        runtime.initialize_genesis(genesis).await.unwrap();
        
        // Create and submit transaction
        let tx = rainsonet_relyo::RelyoTransaction::new(
            sender.address(),
            recipient.address(),
            Amount::from_relyo(10),
            Amount::ZERO, // Devnet has no fee requirement
            Nonce::new(0),
            &sender,
        )
        .unwrap();
        
        let verified = VerifiedTransaction::new(tx).unwrap();
        let tx_id = runtime.submit_transaction(verified).await.unwrap();
        
        // Check mempool or finalized state
        // (Transaction might be immediately processed in single-validator mode)
        assert!(tx_id != Hash::ZERO);
    }
}
