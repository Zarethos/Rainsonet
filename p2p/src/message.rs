//! Network messages for RAINSONET

use rainsonet_core::{Hash, NodeId, Signature, StateRoot, StateVersion, Timestamp};
use serde::{Deserialize, Serialize};

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Message types for RAINSONET protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Handshake message
    Handshake(HandshakeMessage),
    
    /// Transaction broadcast
    Transaction(TransactionMessage),
    
    /// State proposal from validator
    Proposal(ProposalMessage),
    
    /// Vote on a proposal
    Vote(VoteMessage),
    
    /// State sync request
    SyncRequest(SyncRequestMessage),
    
    /// State sync response
    SyncResponse(SyncResponseMessage),
    
    /// Ping for keepalive
    Ping(PingMessage),
    
    /// Pong response
    Pong(PongMessage),
}

impl Message {
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }
    
    pub fn message_type(&self) -> &'static str {
        match self {
            Message::Handshake(_) => "handshake",
            Message::Transaction(_) => "transaction",
            Message::Proposal(_) => "proposal",
            Message::Vote(_) => "vote",
            Message::SyncRequest(_) => "sync_request",
            Message::SyncResponse(_) => "sync_response",
            Message::Ping(_) => "ping",
            Message::Pong(_) => "pong",
        }
    }
}

/// Handshake message for peer connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    pub version: u32,
    pub node_id: NodeId,
    pub is_validator: bool,
    pub state_version: StateVersion,
    pub state_root: StateRoot,
    pub timestamp: Timestamp,
}

impl HandshakeMessage {
    pub fn new(
        node_id: NodeId,
        is_validator: bool,
        state_version: StateVersion,
        state_root: StateRoot,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            node_id,
            is_validator,
            state_version,
            state_root,
            timestamp: Timestamp::now(),
        }
    }
}

/// Transaction broadcast message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMessage {
    pub tx_id: Hash,
    pub tx_data: Vec<u8>,
    pub timestamp: Timestamp,
}

impl TransactionMessage {
    pub fn new(tx_id: Hash, tx_data: Vec<u8>) -> Self {
        Self {
            tx_id,
            tx_data,
            timestamp: Timestamp::now(),
        }
    }
}

/// State proposal from a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalMessage {
    pub proposal_id: Hash,
    pub proposer: NodeId,
    pub state_version: StateVersion,
    pub previous_root: StateRoot,
    pub new_root: StateRoot,
    pub tx_ids: Vec<Hash>,
    pub changes_hash: Hash,
    pub signature: Signature,
    pub timestamp: Timestamp,
}

/// Vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteMessage {
    pub proposal_id: Hash,
    pub voter: NodeId,
    pub approve: bool,
    pub state_version: StateVersion,
    pub state_root: StateRoot,
    pub signature: Signature,
    pub timestamp: Timestamp,
}

/// Request state sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestMessage {
    pub from_version: StateVersion,
    pub to_version: Option<StateVersion>,
    pub requester: NodeId,
    pub timestamp: Timestamp,
}

impl SyncRequestMessage {
    pub fn new(from_version: StateVersion, requester: NodeId) -> Self {
        Self {
            from_version,
            to_version: None,
            requester,
            timestamp: Timestamp::now(),
        }
    }
}

/// State sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponseMessage {
    pub from_version: StateVersion,
    pub to_version: StateVersion,
    pub state_root: StateRoot,
    pub changes: Vec<StateChangeData>,
    pub timestamp: Timestamp,
}

/// State change data for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeData {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>, // None = delete
}

/// Ping message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    pub nonce: u64,
    pub timestamp: Timestamp,
}

impl PingMessage {
    pub fn new() -> Self {
        use rand::Rng;
        Self {
            nonce: rand::thread_rng().gen(),
            timestamp: Timestamp::now(),
        }
    }
}

impl Default for PingMessage {
    fn default() -> Self {
        Self::new()
    }
}

/// Pong response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    pub nonce: u64,
    pub timestamp: Timestamp,
}

impl PongMessage {
    pub fn from_ping(ping: &PingMessage) -> Self {
        Self {
            nonce: ping.nonce,
            timestamp: Timestamp::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_serialization() {
        let msg = Message::Ping(PingMessage::new());
        let bytes = msg.to_bytes();
        let restored = Message::from_bytes(&bytes).unwrap();
        
        assert_eq!(msg.message_type(), restored.message_type());
    }
    
    #[test]
    fn test_handshake_message() {
        let node_id = NodeId::from_bytes([1u8; 32]);
        let msg = HandshakeMessage::new(
            node_id,
            true,
            StateVersion::new(1),
            Hash::ZERO,
        );
        
        assert_eq!(msg.version, PROTOCOL_VERSION);
        assert!(msg.is_validator);
    }
}
