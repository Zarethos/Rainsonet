//! Core traits defining RAINSONET interfaces
//! 
//! These traits define the contracts that all modules must implement.

use crate::types::*;
use async_trait::async_trait;
use std::fmt::Debug;

/// Result type for RAINSONET operations
pub type RainsonetResult<T> = Result<T, crate::error::RainsonetError>;

/// Trait for hashable types
pub trait Hashable {
    /// Compute the hash of this object
    fn hash(&self) -> Hash;
}

/// Trait for signable types
pub trait Signable: Hashable {
    /// Get the bytes to be signed
    fn signing_bytes(&self) -> Vec<u8>;
}

/// Trait for verifiable signatures
pub trait Verifiable: Signable {
    /// Get the signer's public key
    fn signer(&self) -> &PublicKey;
    
    /// Get the signature
    fn signature(&self) -> &Signature;
}

/// Transaction trait for all transaction types
pub trait Transaction: Hashable + Signable + Debug + Clone + Send + Sync {
    /// Get the transaction ID
    fn id(&self) -> TxId {
        self.hash()
    }
    
    /// Get the sender address
    fn sender(&self) -> Address;
    
    /// Get the nonce
    fn nonce(&self) -> Nonce;
    
    /// Get the fee amount
    fn fee(&self) -> Amount;
    
    /// Get the timestamp
    fn timestamp(&self) -> Timestamp;
}

/// State provider trait
#[async_trait]
pub trait StateProvider: Send + Sync {
    /// Get the current state version
    async fn version(&self) -> StateVersion;
    
    /// Get the state root hash
    async fn root(&self) -> StateRoot;
    
    /// Get a value by key
    async fn get(&self, key: &[u8]) -> RainsonetResult<Option<Vec<u8>>>;
    
    /// Check if a key exists
    async fn exists(&self, key: &[u8]) -> RainsonetResult<bool>;
}

/// State mutator trait
#[async_trait]
pub trait StateMutator: StateProvider {
    /// Set a value
    async fn set(&self, key: &[u8], value: &[u8]) -> RainsonetResult<()>;
    
    /// Delete a key
    async fn delete(&self, key: &[u8]) -> RainsonetResult<()>;
    
    /// Apply a batch of changes atomically
    async fn apply_batch(&self, changes: Vec<StateChange>) -> RainsonetResult<StateVersion>;
}

/// State change operation
#[derive(Debug, Clone)]
pub enum StateChange {
    Set { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

/// Transaction validator trait
#[async_trait]
pub trait TransactionValidator<T: Transaction>: Send + Sync {
    /// Validate a transaction against current state
    async fn validate(&self, tx: &T, state: &dyn StateProvider) -> RainsonetResult<()>;
}

/// Transaction executor trait
#[async_trait]
pub trait TransactionExecutor<T: Transaction>: Send + Sync {
    /// Execute a transaction and return state changes
    async fn execute(&self, tx: &T, state: &dyn StateProvider) -> RainsonetResult<Vec<StateChange>>;
}

/// Module trait for payment modules
#[async_trait]
pub trait PaymentModule: Send + Sync {
    /// Module name
    fn name(&self) -> &str;
    
    /// Module version
    fn version(&self) -> &str;
    
    /// Initialize the module
    async fn initialize(&self, state: &dyn StateMutator) -> RainsonetResult<()>;
}

/// Consensus participant trait
#[async_trait]
pub trait ConsensusParticipant: Send + Sync {
    /// Get this node's ID
    fn node_id(&self) -> NodeId;
    
    /// Check if this node is a validator
    fn is_validator(&self) -> bool;
    
    /// Sign a message
    fn sign(&self, message: &[u8]) -> Signature;
    
    /// Verify a signature from another node
    fn verify(&self, node_id: &NodeId, message: &[u8], signature: &Signature) -> bool;
}

/// Vote for consensus
#[derive(Debug, Clone)]
pub struct Vote {
    pub voter: NodeId,
    pub state_version: StateVersion,
    pub state_root: StateRoot,
    pub signature: Signature,
    pub timestamp: Timestamp,
}

/// Consensus engine trait
#[async_trait]
pub trait ConsensusEngine: Send + Sync {
    /// Propose a state update
    async fn propose(&self, changes: Vec<StateChange>) -> RainsonetResult<StateVersion>;
    
    /// Vote on a proposed state
    async fn vote(&self, vote: Vote) -> RainsonetResult<()>;
    
    /// Check if consensus is reached for a version
    async fn is_finalized(&self, version: StateVersion) -> bool;
    
    /// Get the latest finalized version
    async fn latest_finalized(&self) -> StateVersion;
}

/// Network message types
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    /// Transaction broadcast
    Transaction(Vec<u8>),
    /// State update proposal
    Proposal(Vec<u8>),
    /// Vote on proposal
    Vote(Vote),
    /// State sync request
    SyncRequest { from_version: StateVersion },
    /// State sync response
    SyncResponse { changes: Vec<StateChange> },
}

/// Network peer trait
#[async_trait]
pub trait NetworkPeer: Send + Sync {
    /// Get peer's node ID
    fn node_id(&self) -> NodeId;
    
    /// Send a message to this peer
    async fn send(&self, message: NetworkMessage) -> RainsonetResult<()>;
}

/// Network layer trait
#[async_trait]
pub trait NetworkLayer: Send + Sync {
    /// Broadcast a message to all peers
    async fn broadcast(&self, message: NetworkMessage) -> RainsonetResult<()>;
    
    /// Get connected peers
    async fn peers(&self) -> Vec<Box<dyn NetworkPeer>>;
    
    /// Get validator peers
    async fn validators(&self) -> Vec<Box<dyn NetworkPeer>>;
}
