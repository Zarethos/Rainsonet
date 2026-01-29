//! Error types for RAINSONET

use thiserror::Error;

/// Main error type for RAINSONET
#[derive(Error, Debug)]
pub enum RainsonetError {
    // ============ Cryptography Errors ============
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Invalid public key")]
    InvalidPublicKey,
    
    #[error("Invalid private key")]
    InvalidPrivateKey,
    
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    
    #[error("Hash computation failed: {0}")]
    HashFailed(String),
    
    // ============ Transaction Errors ============
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u128, available: u128 },
    
    #[error("Double spend detected for transaction {0}")]
    DoubleSpend(String),
    
    #[error("Transaction expired")]
    TransactionExpired,
    
    #[error("Fee too low: minimum {minimum}, provided {provided}")]
    FeeTooLow { minimum: u128, provided: u128 },
    
    // ============ State Errors ============
    #[error("State not found for key")]
    StateNotFound,
    
    #[error("State version mismatch: expected {expected}, got {got}")]
    StateVersionMismatch { expected: u64, got: u64 },
    
    #[error("State corruption detected: {0}")]
    StateCorruption(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    // ============ Consensus Errors ============
    #[error("Consensus not reached")]
    ConsensusNotReached,
    
    #[error("Invalid vote: {0}")]
    InvalidVote(String),
    
    #[error("Not a validator")]
    NotAValidator,
    
    #[error("Proposal rejected: {0}")]
    ProposalRejected(String),
    
    #[error("Validator set error: {0}")]
    ValidatorSetError(String),
    
    // ============ Network Errors ============
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Message serialization failed: {0}")]
    SerializationError(String),
    
    #[error("Message deserialization failed: {0}")]
    DeserializationError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    // ============ Module Errors ============
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    
    #[error("Module initialization failed: {0}")]
    ModuleInitFailed(String),
    
    #[error("Module error: {0}")]
    ModuleError(String),
    
    // ============ Configuration Errors ============
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    // ============ General Errors ============
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<std::io::Error> for RainsonetError {
    fn from(err: std::io::Error) -> Self {
        RainsonetError::StorageError(err.to_string())
    }
}

impl From<bincode::Error> for RainsonetError {
    fn from(err: bincode::Error) -> Self {
        RainsonetError::SerializationError(err.to_string())
    }
}

impl From<serde_json::Error> for RainsonetError {
    fn from(err: serde_json::Error) -> Self {
        RainsonetError::SerializationError(err.to_string())
    }
}
