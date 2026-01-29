//! In-memory state store for testing and light nodes

use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use rainsonet_core::{
    RainsonetResult, StateChange, StateMutator, StateProvider, StateRoot, StateVersion,
};
use std::sync::Arc;

use crate::store::{StateDiff, StateEntry, StateStore};

/// In-memory state store
pub struct MemoryStateStore {
    data: DashMap<Vec<u8>, Vec<u8>>,
    version: RwLock<StateVersion>,
    history: RwLock<Vec<StateDiff>>,
}

impl MemoryStateStore {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
            version: RwLock::new(StateVersion::new(0)),
            history: RwLock::new(Vec::new()),
        }
    }
    
    pub fn with_data(data: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
        let store = Self::new();
        for (key, value) in data {
            store.data.insert(key, value);
        }
        store
    }
}

impl Default for MemoryStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemoryStateStore {
    fn clone(&self) -> Self {
        let new_store = Self::new();
        for entry in self.data.iter() {
            new_store.data.insert(entry.key().clone(), entry.value().clone());
        }
        *new_store.version.write() = *self.version.read();
        new_store
    }
}

#[async_trait]
impl StateProvider for MemoryStateStore {
    async fn version(&self) -> StateVersion {
        *self.version.read()
    }
    
    async fn root(&self) -> StateRoot {
        self.compute_root().await.unwrap_or(rainsonet_core::Hash::ZERO)
    }
    
    async fn get(&self, key: &[u8]) -> RainsonetResult<Option<Vec<u8>>> {
        Ok(self.data.get(key).map(|v| v.value().clone()))
    }
    
    async fn exists(&self, key: &[u8]) -> RainsonetResult<bool> {
        Ok(self.data.contains_key(key))
    }
}

#[async_trait]
impl StateMutator for MemoryStateStore {
    async fn set(&self, key: &[u8], value: &[u8]) -> RainsonetResult<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> RainsonetResult<()> {
        self.data.remove(key);
        Ok(())
    }
    
    async fn apply_batch(&self, changes: Vec<StateChange>) -> RainsonetResult<StateVersion> {
        let old_version = *self.version.read();
        let mut diff = StateDiff::new(old_version, old_version.next());
        
        for change in changes {
            match change {
                StateChange::Set { key, value } => {
                    diff.add(key.clone(), value.clone());
                    self.data.insert(key, value);
                }
                StateChange::Delete { key } => {
                    diff.remove(key.clone());
                    self.data.remove(&key);
                }
            }
        }
        
        let new_version = old_version.next();
        *self.version.write() = new_version;
        self.history.write().push(diff);
        
        Ok(new_version)
    }
}

#[async_trait]
impl StateStore for MemoryStateStore {
    async fn all_entries(&self) -> RainsonetResult<Vec<StateEntry>> {
        let entries: Vec<StateEntry> = self
            .data
            .iter()
            .map(|entry| StateEntry {
                key: entry.key().clone(),
                value: entry.value().clone(),
            })
            .collect();
        Ok(entries)
    }
    
    async fn snapshot(&self) -> RainsonetResult<Box<dyn StateStore>> {
        Ok(Box::new(self.clone()))
    }
    
    async fn diff(&self, from_version: StateVersion) -> RainsonetResult<StateDiff> {
        let history = self.history.read();
        let current_version = *self.version.read();
        
        let mut combined = StateDiff::new(from_version, current_version);
        
        for diff in history.iter() {
            if diff.from_version.0 >= from_version.0 {
                for (key, value) in &diff.added {
                    combined.add(key.clone(), value.clone());
                }
                for key in &diff.removed {
                    combined.remove(key.clone());
                }
            }
        }
        
        Ok(combined)
    }
}

/// Thread-safe memory store wrapper
pub type SharedMemoryStateStore = Arc<MemoryStateStore>;

/// Create a shared memory state store
pub fn create_memory_store() -> SharedMemoryStateStore {
    Arc::new(MemoryStateStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::AccountState;
    
    #[tokio::test]
    async fn test_memory_store_basic() {
        let store = MemoryStateStore::new();
        
        // Set and get
        store.set(b"key1", b"value1").await.unwrap();
        let value = store.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
        
        // Delete
        store.delete(b"key1").await.unwrap();
        let value = store.get(b"key1").await.unwrap();
        assert_eq!(value, None);
    }
    
    #[tokio::test]
    async fn test_memory_store_batch() {
        let store = MemoryStateStore::new();
        
        let changes = vec![
            StateChange::Set {
                key: b"k1".to_vec(),
                value: b"v1".to_vec(),
            },
            StateChange::Set {
                key: b"k2".to_vec(),
                value: b"v2".to_vec(),
            },
        ];
        
        let version = store.apply_batch(changes).await.unwrap();
        assert_eq!(version.0, 1);
        
        assert!(store.exists(b"k1").await.unwrap());
        assert!(store.exists(b"k2").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_memory_store_account() {
        let store = MemoryStateStore::new();
        
        let address = [1u8; 32];
        let account = AccountState::new(1000, 0);
        
        store.set_account(&address, &account).await.unwrap();
        
        let balance = store.get_balance(&address).await.unwrap();
        assert_eq!(balance, 1000);
        
        let nonce = store.get_nonce(&address).await.unwrap();
        assert_eq!(nonce, 0);
    }
}
