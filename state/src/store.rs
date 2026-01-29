//! Core state store traits and types

use async_trait::async_trait;
use rainsonet_core::{
    Hash, RainsonetError, RainsonetResult, StateChange, StateMutator, StateProvider,
    StateRoot, StateVersion,
};
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
    
    // Sort entries by key for deterministic ordering
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.key.cmp(&b.key));
    
    // Compute leaf hashes
    let leaves: Vec<Hash> = sorted.iter().map(|e| e.hash()).collect();
    
    // Compute merkle root
    merkle_root(&leaves)
}

/// Batch of state changes with metadata
#[derive(Debug, Clone)]
pub struct StateBatch {
    pub version: StateVersion,
    pub changes: Vec<StateChange>,
    pub previous_root: StateRoot,
    pub new_root: StateRoot,
}

impl StateBatch {
    pub fn new(
        version: StateVersion,
        changes: Vec<StateChange>,
        previous_root: StateRoot,
        new_root: StateRoot,
    ) -> Self {
        Self {
            version,
            changes,
            previous_root,
            new_root,
        }
    }
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

/// Abstract state store interface
#[async_trait]
pub trait StateStore: StateProvider + StateMutator {
    /// Get account state
    async fn get_account(&self, address: &[u8]) -> RainsonetResult<Option<AccountState>> {
        let key = account_key(address);
        match self.get(&key).await? {
            Some(bytes) => Ok(Some(AccountState::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }
    
    /// Set account state
    async fn set_account(&self, address: &[u8], state: &AccountState) -> RainsonetResult<()> {
        let key = account_key(address);
        self.set(&key, &state.to_bytes()).await
    }
    
    /// Get balance
    async fn get_balance(&self, address: &[u8]) -> RainsonetResult<u128> {
        Ok(self
            .get_account(address)
            .await?
            .map(|a| a.balance)
            .unwrap_or(0))
    }
    
    /// Get nonce
    async fn get_nonce(&self, address: &[u8]) -> RainsonetResult<u64> {
        Ok(self
            .get_account(address)
            .await?
            .map(|a| a.nonce)
            .unwrap_or(0))
    }
    
    /// Get all entries for state root computation
    async fn all_entries(&self) -> RainsonetResult<Vec<StateEntry>>;
    
    /// Compute current state root
    async fn compute_root(&self) -> RainsonetResult<StateRoot> {
        let entries = self.all_entries().await?;
        Ok(compute_state_root(&entries))
    }
    
    /// Create a snapshot
    async fn snapshot(&self) -> RainsonetResult<Box<dyn StateStore>>;
    
    /// Get diff between versions
    async fn diff(&self, from_version: StateVersion) -> RainsonetResult<StateDiff>;
}

/// Key prefix for accounts
const ACCOUNT_PREFIX: &[u8] = b"account:";

/// Build account key
pub fn account_key(address: &[u8]) -> Vec<u8> {
    let mut key = ACCOUNT_PREFIX.to_vec();
    key.extend_from_slice(address);
    key
}

/// Parse account key
pub fn parse_account_key(key: &[u8]) -> Option<Vec<u8>> {
    if key.starts_with(ACCOUNT_PREFIX) {
        Some(key[ACCOUNT_PREFIX.len()..].to_vec())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_account_state_serialization() {
        let state = AccountState::new(1000, 5);
        let bytes = state.to_bytes();
        let restored = AccountState::from_bytes(&bytes).unwrap();
        
        assert_eq!(state.balance, restored.balance);
        assert_eq!(state.nonce, restored.nonce);
    }
    
    #[test]
    fn test_state_root_deterministic() {
        let entries = vec![
            StateEntry {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            },
            StateEntry {
                key: b"key2".to_vec(),
                value: b"value2".to_vec(),
            },
        ];
        
        let root1 = compute_state_root(&entries);
        let root2 = compute_state_root(&entries);
        
        assert_eq!(root1, root2);
    }
    
    #[test]
    fn test_account_key() {
        let address = [1u8; 32];
        let key = account_key(&address);
        let parsed = parse_account_key(&key).unwrap();
        
        assert_eq!(&address[..], &parsed[..]);
    }
}
