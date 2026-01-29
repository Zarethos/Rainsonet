//! In-memory state store for testing and light nodes

use dashmap::DashMap;
use parking_lot::RwLock;
use rainsonet_core::{Hash, RainsonetResult, StateRoot, StateVersion};
use std::sync::Arc;

use crate::store::{
    account_key, compute_state_root, AccountState, StateChangeOp, StateDiff, StateEntry,
};

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
    
    pub fn version(&self) -> StateVersion {
        *self.version.read()
    }
    
    pub fn root(&self) -> StateRoot {
        self.compute_root().unwrap_or(Hash::ZERO)
    }
    
    pub fn get(&self, key: &[u8]) -> RainsonetResult<Option<Vec<u8>>> {
        Ok(self.data.get(key).map(|v| v.value().clone()))
    }
    
    pub fn exists(&self, key: &[u8]) -> RainsonetResult<bool> {
        Ok(self.data.contains_key(key))
    }
    
    pub fn set(&self, key: &[u8], value: &[u8]) -> RainsonetResult<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }
    
    pub fn delete(&self, key: &[u8]) -> RainsonetResult<()> {
        self.data.remove(key);
        Ok(())
    }
    
    pub fn apply_batch(&self, changes: Vec<StateChangeOp>) -> RainsonetResult<StateVersion> {
        let old_version = *self.version.read();
        let mut diff = StateDiff::new(old_version, old_version.next());
        
        for change in changes {
            match change {
                StateChangeOp::Set { key, value } => {
                    diff.add(key.clone(), value.clone());
                    self.data.insert(key, value);
                }
                StateChangeOp::Delete { key } => {
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
    
    pub fn all_entries(&self) -> RainsonetResult<Vec<StateEntry>> {
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
    
    pub fn compute_root(&self) -> RainsonetResult<StateRoot> {
        let entries = self.all_entries()?;
        Ok(compute_state_root(&entries))
    }
    
    pub fn snapshot(&self) -> Self {
        let new_store = Self::new();
        for entry in self.data.iter() {
            new_store.data.insert(entry.key().clone(), entry.value().clone());
        }
        *new_store.version.write() = *self.version.read();
        new_store
    }
    
    pub fn diff(&self, from_version: StateVersion) -> RainsonetResult<StateDiff> {
        let history = self.history.read();
        let current_version = *self.version.read();
        
        let mut combined = StateDiff::new(from_version, current_version);
        
        for d in history.iter() {
            if d.from_version.0 >= from_version.0 {
                for (key, value) in &d.added {
                    combined.add(key.clone(), value.clone());
                }
                for key in &d.removed {
                    combined.remove(key.clone());
                }
            }
        }
        
        Ok(combined)
    }
    
    // Account-specific methods
    
    pub fn get_account(&self, address: &[u8]) -> RainsonetResult<Option<AccountState>> {
        let key = account_key(address);
        match self.get(&key)? {
            Some(bytes) => Ok(Some(AccountState::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }
    
    pub fn set_account(&self, address: &[u8], state: &AccountState) -> RainsonetResult<()> {
        let key = account_key(address);
        self.set(&key, &state.to_bytes())
    }
    
    pub fn get_balance(&self, address: &[u8]) -> RainsonetResult<u128> {
        Ok(self.get_account(address)?.map(|a| a.balance).unwrap_or(0))
    }
    
    pub fn get_nonce(&self, address: &[u8]) -> RainsonetResult<u64> {
        Ok(self.get_account(address)?.map(|a| a.nonce).unwrap_or(0))
    }
}

impl Default for MemoryStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemoryStateStore {
    fn clone(&self) -> Self {
        self.snapshot()
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
    
    #[test]
    fn test_memory_store_basic() {
        let store = MemoryStateStore::new();
        
        store.set(b"key1", b"value1").unwrap();
        let value = store.get(b"key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
        
        store.delete(b"key1").unwrap();
        let value = store.get(b"key1").unwrap();
        assert_eq!(value, None);
    }
    
    #[test]
    fn test_memory_store_batch() {
        let store = MemoryStateStore::new();
        
        let changes = vec![
            StateChangeOp::Set {
                key: b"k1".to_vec(),
                value: b"v1".to_vec(),
            },
            StateChangeOp::Set {
                key: b"k2".to_vec(),
                value: b"v2".to_vec(),
            },
        ];
        
        let version = store.apply_batch(changes).unwrap();
        assert_eq!(version.0, 1);
        
        assert!(store.exists(b"k1").unwrap());
        assert!(store.exists(b"k2").unwrap());
    }
    
    #[test]
    fn test_account_state() {
        let store = MemoryStateStore::new();
        let addr = [1u8; 32];
        
        let state = AccountState::new(1000, 5);
        store.set_account(&addr, &state).unwrap();
        
        let loaded = store.get_account(&addr).unwrap().unwrap();
        assert_eq!(loaded.balance, 1000);
        assert_eq!(loaded.nonce, 5);
    }
}
