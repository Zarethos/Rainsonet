//! State snapshots for synchronization and backup

use rainsonet_core::{RainsonetError, RainsonetResult, StateRoot, StateVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::store::{compute_state_root, StateEntry};

/// A complete snapshot of the state at a specific version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub version: StateVersion,
    pub root: StateRoot,
    pub entries: BTreeMap<Vec<u8>, Vec<u8>>,
    pub created_at: u64,
}

impl StateSnapshot {
    /// Create a new snapshot
    pub fn new(version: StateVersion, entries: Vec<StateEntry>) -> Self {
        let state_entries: Vec<StateEntry> = entries
            .iter()
            .map(|e| StateEntry {
                key: e.key.clone(),
                value: e.value.clone(),
            })
            .collect();
        
        let root = compute_state_root(&state_entries);
        
        let entries_map: BTreeMap<Vec<u8>, Vec<u8>> = entries
            .into_iter()
            .map(|e| (e.key, e.value))
            .collect();
        
        Self {
            version,
            root,
            entries: entries_map,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
    
    /// Verify the snapshot's integrity
    pub fn verify(&self) -> bool {
        let entries: Vec<StateEntry> = self
            .entries
            .iter()
            .map(|(k, v)| StateEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();
        
        let computed_root = compute_state_root(&entries);
        computed_root == self.root
    }
    
    /// Get a value from the snapshot
    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.entries.get(key)
    }
    
    /// Check if a key exists
    pub fn contains(&self, key: &[u8]) -> bool {
        self.entries.contains_key(key)
    }
    
    /// Get all entries
    pub fn all_entries(&self) -> Vec<StateEntry> {
        self.entries
            .iter()
            .map(|(k, v)| StateEntry {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> RainsonetResult<Vec<u8>> {
        bincode::serialize(self).map_err(|e| RainsonetError::SerializationError(e.to_string()))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> RainsonetResult<Self> {
        bincode::deserialize(bytes).map_err(|e| RainsonetError::DeserializationError(e.to_string()))
    }
    
    /// Get size in bytes (approximate)
    pub fn size(&self) -> usize {
        self.entries
            .iter()
            .map(|(k, v)| k.len() + v.len())
            .sum()
    }
    
    /// Number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Snapshot manager for creating and restoring snapshots
pub struct SnapshotManager {
    max_snapshots: usize,
    snapshots: Vec<StateSnapshot>,
}

impl SnapshotManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            max_snapshots,
            snapshots: Vec::new(),
        }
    }
    
    /// Add a snapshot
    pub fn add(&mut self, snapshot: StateSnapshot) {
        self.snapshots.push(snapshot);
        
        // Remove old snapshots if over limit
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
    
    /// Get the closest snapshot before a version
    pub fn closest_before(&self, version: StateVersion) -> Option<&StateSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.version.0 <= version.0)
            .max_by_key(|s| s.version.0)
    }
    
    /// List all snapshot versions
    pub fn versions(&self) -> Vec<StateVersion> {
        self.snapshots.iter().map(|s| s.version).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_snapshot_create_and_verify() {
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
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot.get(b"key1"), Some(&b"value1".to_vec()));
    }
    
    #[test]
    fn test_snapshot_serialization() {
        let entries = vec![StateEntry {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        }];
        
        let snapshot = StateSnapshot::new(StateVersion::new(1), entries);
        let bytes = snapshot.to_bytes().unwrap();
        let restored = StateSnapshot::from_bytes(&bytes).unwrap();
        
        assert_eq!(snapshot.version, restored.version);
        assert_eq!(snapshot.root, restored.root);
        assert!(restored.verify());
    }
    
    #[test]
    fn test_snapshot_manager() {
        let mut manager = SnapshotManager::new(3);
        
        for i in 1..=5 {
            let snapshot = StateSnapshot::new(StateVersion::new(i), vec![]);
            manager.add(snapshot);
        }
        
        // Should only keep last 3
        assert_eq!(manager.versions().len(), 3);
        assert_eq!(manager.latest().unwrap().version.0, 5);
        assert!(manager.at_version(StateVersion::new(2)).is_none());
        assert!(manager.at_version(StateVersion::new(4)).is_some());
    }
}
