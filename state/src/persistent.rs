//! Persistent state store using sled embedded database

use async_trait::async_trait;
use parking_lot::RwLock;
use rainsonet_core::{
    RainsonetError, RainsonetResult, StateChange, StateMutator, StateProvider, StateRoot,
    StateVersion,
};
use sled::Db;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::store::{StateDiff, StateEntry, StateStore};

const VERSION_KEY: &[u8] = b"__version__";
const DIFF_PREFIX: &[u8] = b"__diff__:";

/// Persistent state store using sled
pub struct PersistentStateStore {
    db: Db,
    version: RwLock<StateVersion>,
}

impl PersistentStateStore {
    /// Open or create a persistent store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> RainsonetResult<Self> {
        let db = sled::open(path).map_err(|e| RainsonetError::StorageError(e.to_string()))?;
        
        // Load version
        let version = match db.get(VERSION_KEY) {
            Ok(Some(bytes)) => {
                let v = u64::from_le_bytes(bytes.as_ref().try_into().unwrap_or([0; 8]));
                StateVersion::new(v)
            }
            _ => StateVersion::new(0),
        };
        
        info!("Opened persistent store at version {}", version);
        
        Ok(Self {
            db,
            version: RwLock::new(version),
        })
    }
    
    /// Flush to disk
    pub fn flush(&self) -> RainsonetResult<()> {
        self.db
            .flush()
            .map_err(|e| RainsonetError::StorageError(e.to_string()))?;
        Ok(())
    }
    
    fn save_version(&self, version: StateVersion) -> RainsonetResult<()> {
        self.db
            .insert(VERSION_KEY, &version.0.to_le_bytes())
            .map_err(|e| RainsonetError::StorageError(e.to_string()))?;
        Ok(())
    }
    
    fn save_diff(&self, diff: &StateDiff) -> RainsonetResult<()> {
        let key = format!("{}:{}", String::from_utf8_lossy(DIFF_PREFIX), diff.to_version.0);
        let value = bincode::serialize(diff).map_err(|e| RainsonetError::SerializationError(e.to_string()))?;
        self.db
            .insert(key.as_bytes(), value)
            .map_err(|e| RainsonetError::StorageError(e.to_string()))?;
        Ok(())
    }
    
    fn is_internal_key(key: &[u8]) -> bool {
        key.starts_with(b"__")
    }
}

#[async_trait]
impl StateProvider for PersistentStateStore {
    async fn version(&self) -> StateVersion {
        *self.version.read()
    }
    
    async fn root(&self) -> StateRoot {
        self.compute_root().await.unwrap_or(rainsonet_core::Hash::ZERO)
    }
    
    async fn get(&self, key: &[u8]) -> RainsonetResult<Option<Vec<u8>>> {
        match self.db.get(key) {
            Ok(Some(ivec)) => Ok(Some(ivec.to_vec())),
            Ok(None) => Ok(None),
            Err(e) => Err(RainsonetError::StorageError(e.to_string())),
        }
    }
    
    async fn exists(&self, key: &[u8]) -> RainsonetResult<bool> {
        match self.db.contains_key(key) {
            Ok(exists) => Ok(exists),
            Err(e) => Err(RainsonetError::StorageError(e.to_string())),
        }
    }
}

#[async_trait]
impl StateMutator for PersistentStateStore {
    async fn set(&self, key: &[u8], value: &[u8]) -> RainsonetResult<()> {
        self.db
            .insert(key, value)
            .map_err(|e| RainsonetError::StorageError(e.to_string()))?;
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> RainsonetResult<()> {
        self.db
            .remove(key)
            .map_err(|e| RainsonetError::StorageError(e.to_string()))?;
        Ok(())
    }
    
    async fn apply_batch(&self, changes: Vec<StateChange>) -> RainsonetResult<StateVersion> {
        let old_version = *self.version.read();
        let new_version = old_version.next();
        let mut diff = StateDiff::new(old_version, new_version);
        
        // Create batch
        let mut batch = sled::Batch::default();
        
        for change in changes {
            match change {
                StateChange::Set { key, value } => {
                    diff.add(key.clone(), value.clone());
                    batch.insert(key, value);
                }
                StateChange::Delete { key } => {
                    diff.remove(key.clone());
                    batch.remove(key);
                }
            }
        }
        
        // Apply batch atomically
        self.db
            .apply_batch(batch)
            .map_err(|e| RainsonetError::StorageError(e.to_string()))?;
        
        // Save version and diff
        self.save_version(new_version)?;
        self.save_diff(&diff)?;
        
        *self.version.write() = new_version;
        
        debug!("Applied batch, new version: {}", new_version);
        
        Ok(new_version)
    }
}

#[async_trait]
impl StateStore for PersistentStateStore {
    async fn all_entries(&self) -> RainsonetResult<Vec<StateEntry>> {
        let mut entries = Vec::new();
        
        for item in self.db.iter() {
            match item {
                Ok((key, value)) => {
                    if !Self::is_internal_key(&key) {
                        entries.push(StateEntry {
                            key: key.to_vec(),
                            value: value.to_vec(),
                        });
                    }
                }
                Err(e) => {
                    error!("Error iterating state: {}", e);
                    return Err(RainsonetError::StorageError(e.to_string()));
                }
            }
        }
        
        Ok(entries)
    }
    
    async fn snapshot(&self) -> RainsonetResult<Box<dyn StateStore>> {
        // For persistent store, we create an in-memory snapshot
        use crate::memory::MemoryStateStore;
        
        let entries = self.all_entries().await?;
        let data: Vec<(Vec<u8>, Vec<u8>)> = entries
            .into_iter()
            .map(|e| (e.key, e.value))
            .collect();
        
        Ok(Box::new(MemoryStateStore::with_data(data)))
    }
    
    async fn diff(&self, from_version: StateVersion) -> RainsonetResult<StateDiff> {
        let current_version = *self.version.read();
        let mut combined = StateDiff::new(from_version, current_version);
        
        // Load diffs from storage
        for v in (from_version.0 + 1)..=current_version.0 {
            let key = format!("{}:{}", String::from_utf8_lossy(DIFF_PREFIX), v);
            if let Ok(Some(bytes)) = self.db.get(key.as_bytes()) {
                if let Ok(diff) = bincode::deserialize::<StateDiff>(&bytes) {
                    for (k, val) in diff.added {
                        combined.add(k, val);
                    }
                    for k in diff.removed {
                        combined.remove(k);
                    }
                }
            }
        }
        
        Ok(combined)
    }
}

/// Shared persistent store
pub type SharedPersistentStateStore = Arc<PersistentStateStore>;

/// Create a shared persistent store
pub fn create_persistent_store<P: AsRef<Path>>(path: P) -> RainsonetResult<SharedPersistentStateStore> {
    Ok(Arc::new(PersistentStateStore::open(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_persistent_store_basic() {
        let dir = tempdir().unwrap();
        let store = PersistentStateStore::open(dir.path().join("test.db")).unwrap();
        
        // Set and get
        store.set(b"key1", b"value1").await.unwrap();
        let value = store.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
        
        store.flush().unwrap();
    }
    
    #[tokio::test]
    async fn test_persistent_store_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("persist.db");
        
        // Write data
        {
            let store = PersistentStateStore::open(&path).unwrap();
            store.set(b"key1", b"value1").await.unwrap();
            store.flush().unwrap();
        }
        
        // Read data after reopening
        {
            let store = PersistentStateStore::open(&path).unwrap();
            let value = store.get(b"key1").await.unwrap();
            assert_eq!(value, Some(b"value1".to_vec()));
        }
    }
}
