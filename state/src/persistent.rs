//! Persistent state store using sled database

use parking_lot::RwLock;
use rainsonet_core::{Hash, RainsonetError, RainsonetResult, StateRoot, StateVersion};
use sled::{Db, Tree};
use std::path::Path;
use std::sync::Arc;

use crate::store::{
    account_key, compute_state_root, AccountState, StateChangeOp, StateDiff, StateEntry,
};
use crate::memory::MemoryStateStore;

const STATE_TREE: &str = "state";
const META_TREE: &str = "meta";
const HISTORY_TREE: &str = "history";
const VERSION_KEY: &[u8] = b"version";

/// Persistent state store backed by sled database
pub struct PersistentStateStore {
    db: Db,
    state: Tree,
    meta: Tree,
    history: Tree,
    version: RwLock<StateVersion>,
}

impl PersistentStateStore {
    pub fn open<P: AsRef<Path>>(path: P) -> RainsonetResult<Self> {
        let db = sled::open(path).map_err(|e| RainsonetError::Internal(e.to_string()))?;
        
        let state = db
            .open_tree(STATE_TREE)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        let meta = db
            .open_tree(META_TREE)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        let history = db
            .open_tree(HISTORY_TREE)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        
        // Load version from disk or start at 0
        let version = match meta.get(VERSION_KEY).map_err(|e| RainsonetError::Internal(e.to_string()))? {
            Some(bytes) => {
                let v = u64::from_le_bytes(bytes.as_ref().try_into().unwrap_or([0; 8]));
                StateVersion::new(v)
            }
            None => StateVersion::new(0),
        };
        
        Ok(Self {
            db,
            state,
            meta,
            history,
            version: RwLock::new(version),
        })
    }
    
    pub fn version(&self) -> StateVersion {
        *self.version.read()
    }
    
    pub fn root(&self) -> StateRoot {
        self.compute_root().unwrap_or(Hash::ZERO)
    }
    
    pub fn get(&self, key: &[u8]) -> RainsonetResult<Option<Vec<u8>>> {
        self.state
            .get(key)
            .map(|opt| opt.map(|v| v.to_vec()))
            .map_err(|e| RainsonetError::Internal(e.to_string()))
    }
    
    pub fn exists(&self, key: &[u8]) -> RainsonetResult<bool> {
        self.state
            .contains_key(key)
            .map_err(|e| RainsonetError::Internal(e.to_string()))
    }
    
    pub fn set(&self, key: &[u8], value: &[u8]) -> RainsonetResult<()> {
        self.state
            .insert(key, value)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        Ok(())
    }
    
    pub fn delete(&self, key: &[u8]) -> RainsonetResult<()> {
        self.state
            .remove(key)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        Ok(())
    }
    
    pub fn apply_batch(&self, changes: Vec<StateChangeOp>) -> RainsonetResult<StateVersion> {
        let old_version = *self.version.read();
        let new_version = old_version.next();
        
        // Create a batch for atomic writes
        let mut batch = sled::Batch::default();
        let mut diff = StateDiff::new(old_version, new_version);
        
        for change in changes {
            match change {
                StateChangeOp::Set { key, value } => {
                    diff.add(key.clone(), value.clone());
                    batch.insert(key.as_slice(), value.as_slice());
                }
                StateChangeOp::Delete { key } => {
                    diff.remove(key.clone());
                    batch.remove(key.as_slice());
                }
            }
        }
        
        // Apply state changes atomically
        self.state
            .apply_batch(batch)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        
        // Save new version
        self.meta
            .insert(VERSION_KEY, &new_version.0.to_le_bytes())
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        
        // Save diff to history
        let diff_key = old_version.0.to_le_bytes();
        let diff_bytes = serde_json::to_vec(&diff)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        self.history
            .insert(&diff_key, diff_bytes)
            .map_err(|e| RainsonetError::Internal(e.to_string()))?;
        
        // Flush to disk
        self.db.flush().map_err(|e| RainsonetError::Internal(e.to_string()))?;
        
        // Update in-memory version
        *self.version.write() = new_version;
        
        Ok(new_version)
    }
    
    pub fn all_entries(&self) -> RainsonetResult<Vec<StateEntry>> {
        let entries: Result<Vec<StateEntry>, _> = self
            .state
            .iter()
            .map(|result| {
                result.map(|(key, value)| StateEntry {
                    key: key.to_vec(),
                    value: value.to_vec(),
                })
            })
            .collect();
        
        entries.map_err(|e| RainsonetError::Internal(e.to_string()))
    }
    
    pub fn compute_root(&self) -> RainsonetResult<StateRoot> {
        let entries = self.all_entries()?;
        Ok(compute_state_root(&entries))
    }
    
    pub fn snapshot(&self) -> MemoryStateStore {
        let entries = self.all_entries().unwrap_or_default();
        let data: Vec<(Vec<u8>, Vec<u8>)> = entries
            .into_iter()
            .map(|e| (e.key, e.value))
            .collect();
        MemoryStateStore::with_data(data)
    }
    
    pub fn diff(&self, from_version: StateVersion) -> RainsonetResult<StateDiff> {
        let current_version = *self.version.read();
        let mut combined = StateDiff::new(from_version, current_version);
        
        // Read all diffs from history
        for result in self.history.range(from_version.0.to_le_bytes()..) {
            let (_, diff_bytes) = result.map_err(|e| RainsonetError::Internal(e.to_string()))?;
            let d: StateDiff = serde_json::from_slice(&diff_bytes)
                .map_err(|e| RainsonetError::Internal(e.to_string()))?;
            
            for (key, value) in d.added {
                combined.add(key, value);
            }
            for key in d.removed {
                combined.remove(key);
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
    
    /// Compact the database
    pub fn compact(&self) -> RainsonetResult<()> {
        // Sled doesn't have explicit compaction, but we can flush
        self.db.flush().map_err(|e| RainsonetError::Internal(e.to_string()))
    }
    
    /// Get database size estimate
    pub fn size_estimate(&self) -> RainsonetResult<u64> {
        Ok(self.state.len() as u64)
    }
}

/// Thread-safe persistent store wrapper
pub type SharedPersistentStateStore = Arc<PersistentStateStore>;

/// Create a shared persistent state store
pub fn create_persistent_store<P: AsRef<Path>>(path: P) -> RainsonetResult<SharedPersistentStateStore> {
    Ok(Arc::new(PersistentStateStore::open(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_persistent_store_basic() {
        let tmp = TempDir::new().unwrap();
        let store = PersistentStateStore::open(tmp.path()).unwrap();
        
        store.set(b"key1", b"value1").unwrap();
        let value = store.get(b"key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
        
        store.delete(b"key1").unwrap();
        let value = store.get(b"key1").unwrap();
        assert_eq!(value, None);
    }
    
    #[test]
    fn test_persistent_store_reopen() {
        let tmp = TempDir::new().unwrap();
        
        // Write data
        {
            let store = PersistentStateStore::open(tmp.path()).unwrap();
            store.set(b"key1", b"value1").unwrap();
            let changes = vec![StateChangeOp::Set {
                key: b"k2".to_vec(),
                value: b"v2".to_vec(),
            }];
            store.apply_batch(changes).unwrap();
        }
        
        // Reopen and verify
        {
            let store = PersistentStateStore::open(tmp.path()).unwrap();
            assert_eq!(store.get(b"key1").unwrap(), Some(b"value1".to_vec()));
            assert_eq!(store.get(b"k2").unwrap(), Some(b"v2".to_vec()));
            assert_eq!(store.version().0, 1);
        }
    }
}
