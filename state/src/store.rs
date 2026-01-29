//! Core state store traits and types

use rainsonet_core::{Hash, RainsonetError, RainsonetResult, StateRoot, StateVersion};
use rainsonet_crypto::hashing::{hash, merkle_root};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Account state for RELYO
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountState {
    pub balance: u128,
    pub nonce: u64,
}

impl AccountState {
    pub fn new(balance: u128, nonce: u64) -> Self {
        Self { balance, nonce }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }
    
    pub fn from_bytes(bytes: &[u8]) -> RainsonetResult<Self> {
        bincode::deserialize(bytes).map_err(|e| RainsonetError::DeserializationError(e.to_string()))
    }
}

/// State entry for merkle tree computation
#[derive(Debug, Clone)]
pub struct StateEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl StateEntry {
    pub fn hash(&self) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(&self.key);
        data.extend_from_slice(&self.value);
        hash(&data)
    }
}

/// Compute state root from entries
pub fn compute_state_root(entries: &[StateEntry]) -> StateRoot {
    if entries.is_empty() {
        return Hash::ZERO;
    }
    
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.key.cmp(&b.key));
    
    let leaves: Vec<Hash> = sorted.iter().map(|e| e.hash()).collect();
    
    merkle_root(&leaves)
}

/// Batch of state changes with metadata
#[derive(Debug, Clone)]
pub struct StateBatch {
    pub version: StateVersion,
    pub changes: Vec<StateChangeOp>,
    pub previous_root: StateRoot,
    pub new_root: StateRoot,
}

/// State change operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChangeOp {
    Set { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

/// State diff for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    pub from_version: StateVersion,
    pub to_version: StateVersion,
    pub added: BTreeMap<Vec<u8>, Vec<u8>>,
    pub removed: Vec<Vec<u8>>,
}

impl StateDiff {
    pub fn new(from_version: StateVersion, to_version: StateVersion) -> Self {
        Self {
            from_version,
            to_version,
            added: BTreeMap::new(),
            removed: Vec::new(),
        }
    }
    
    pub fn add(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.added.insert(key, value);
    }
    
    pub fn remove(&mut self, key: Vec<u8>) {
        self.removed.push(key);
    }
    
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// Key prefix for accounts
pub const ACCOUNT_PREFIX: &[u8] = b"account:";

/// Build account key
pub fn account_key(address: &[u8]) -> Vec<u8> {
    let mut key = ACCOUNT_PREFIX.to_vec();
    key.extend_from_slice(address);
    key
}

/// Parse account key to address
pub fn parse_account_key(key: &[u8]) -> Option<Vec<u8>> {
    if key.starts_with(ACCOUNT_PREFIX) {
        Some(key[ACCOUNT_PREFIX.len()..].to_vec())
    } else {
        None
    }
}
