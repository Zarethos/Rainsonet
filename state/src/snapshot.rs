//! State snapshot functionality

use rainsonet_core::{Hash, RainsonetResult, StateRoot, StateVersion};
use serde::{Deserialize, Serialize};

use crate::memory::MemoryStateStore;
use crate::store::{compute_state_root, StateEntry};

/// A complete state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Version at time of snapshot
    pub version: StateVersion,
    /// State root hash
    pub root: StateRoot,
    /// All state entries
    pub entries: Vec<StateEntry>,
    /// Timestamp of snapshot creation
    pub timestamp: u64,
}

impl StateSnapshot {
    /// Create a new snapshot from entries
    pub fn new(version: StateVersion, entries: Vec<StateEntry>) -> Self {
        let root = compute_state_root(&entries);
        Self {
            version,
            root,
            entries,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
    
    /// Create a snapshot from a memory store
    pub fn from_memory_store(store: &MemoryStateStore) -> RainsonetResult<Self> {
        let version = store.version();
        let entries = store.all_entries()?;
        Ok(Self::new(version, entries))
    }
    
    /// Verify snapshot integrity
    pub fn verify(&self) -> bool {
        let computed_root = compute_state_root(&self.entries);
        computed_root == self.root
    }
    
    /// Restore snapshot to a memory store
    pub fn restore(&self) -> MemoryStateStore {
        let data: Vec<(Vec<u8>, Vec<u8>)> = self
            .entries
            .iter()
            .map(|e| (e.key.clone(), e.value.clone()))
            .collect();
        MemoryStateStore::with_data(data)
    }
    
    /// Serialize snapshot to bytes
    pub fn to_bytes(&self) -> RainsonetResult<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| rainsonet_core::RainsonetError::Serialization(e.to_string()))
    }
    
    /// Deserialize snapshot from bytes
    pub fn from_bytes(bytes: &[u8]) -> RainsonetResult<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| rainsonet_core::RainsonetError::Serialization(e.to_string()))
    }
    
    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if snapshot is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Snapshot manager for incremental snapshots
pub struct SnapshotManager {
    snapshots: Vec<StateSnapshot>,
    max_snapshots: usize,
}

impl SnapshotManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Vec::new(),
            max_snapshots,
        }
    }
    
    /// Add a new snapshot
    pub fn add(&mut self, snapshot: StateSnapshot) {
        self.snapshots.push(snapshot);
        
        // Prune old snapshots if needed
        while self.snapshots.len() > self.max_snapshots {
            self.snapshots.remove(0);
        }
    }
    
    /// Get the latest snapshot
    pub fn latest(&self) -> Option<&StateSnapshot> {
        self.snapshots.last()
    }
    
    /// Get snapshot at a specific version
    pub fn at_version(&self, version: StateVersion) -> Option<&StateSnapshot> {
        self.snapshots.iter().find(|s| s.version == version)
    }
    
    /// Get snapshot closest to but not exceeding a version
    pub fn closest_to(&self, version: StateVersion) -> Option<&StateSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.version.0 <= version.0)
            .max_by_key(|s| s.version.0)
    }
    
    /// List all available snapshot versions
    pub fn versions(&self) -> Vec<StateVersion> {
        self.snapshots.iter().map(|s| s.version).collect()
    }
    
    /// Clear all snapshots
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_snapshot_roundtrip() {
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
        
        let snapshot = StateSnapshot::new(StateVersion::new(1), entries);
        assert!(snapshot.verify());
        
        let bytes = snapshot.to_bytes().unwrap();
        let restored = StateSnapshot::from_bytes(&bytes).unwrap();
        
        assert_eq!(restored.version, snapshot.version);
        assert_eq!(restored.root, snapshot.root);
        assert!(restored.verify());
    }
    
    #[test]
    fn test_snapshot_manager() {
        let mut manager = SnapshotManager::new(3);
        
        for i in 0..5 {
            let snapshot = StateSnapshot::new(StateVersion::new(i), vec![]);
            manager.add(snapshot);
        }
        
        // Should only keep last 3
        assert_eq!(manager.versions().len(), 3);
        assert_eq!(manager.latest().unwrap().version.0, 4);
    }
}
