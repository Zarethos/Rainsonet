//! Peer management

use rainsonet_core::{NodeId, StateRoot, StateVersion, Timestamp};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use parking_lot::RwLock;

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub address: Option<SocketAddr>,
    pub is_validator: bool,
    pub state_version: StateVersion,
    pub state_root: StateRoot,
    pub connected_at: Timestamp,
    pub last_seen: Timestamp,
    pub latency_ms: Option<u64>,
}

impl PeerInfo {
    pub fn new(node_id: NodeId, is_validator: bool) -> Self {
        let now = Timestamp::now();
        Self {
            node_id,
            address: None,
            is_validator,
            state_version: StateVersion::new(0),
            state_root: rainsonet_core::Hash::ZERO,
            connected_at: now,
            last_seen: now,
            latency_ms: None,
        }
    }
    
    pub fn update_last_seen(&mut self) {
        self.last_seen = Timestamp::now();
    }
    
    pub fn update_state(&mut self, version: StateVersion, root: StateRoot) {
        self.state_version = version;
        self.state_root = root;
        self.update_last_seen();
    }
    
    pub fn is_stale(&self, timeout_ms: u64) -> bool {
        let now = Timestamp::now();
        now.as_millis() - self.last_seen.as_millis() > timeout_ms
    }
}

/// Peer manager for tracking connected peers
pub struct PeerManager {
    peers: RwLock<HashMap<NodeId, PeerInfo>>,
    max_peers: usize,
}

impl PeerManager {
    pub fn new(max_peers: usize) -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            max_peers,
        }
    }
    
    /// Add or update a peer
    pub fn add_peer(&self, info: PeerInfo) -> bool {
        let mut peers = self.peers.write();
        
        if peers.len() >= self.max_peers && !peers.contains_key(&info.node_id) {
            return false;
        }
        
        peers.insert(info.node_id, info);
        true
    }
    
    /// Remove a peer
    pub fn remove_peer(&self, node_id: &NodeId) {
        self.peers.write().remove(node_id);
    }
    
    /// Get peer info
    pub fn get_peer(&self, node_id: &NodeId) -> Option<PeerInfo> {
        self.peers.read().get(node_id).cloned()
    }
    
    /// Update peer's last seen time
    pub fn update_last_seen(&self, node_id: &NodeId) {
        if let Some(peer) = self.peers.write().get_mut(node_id) {
            peer.update_last_seen();
        }
    }
    
    /// Update peer's state
    pub fn update_peer_state(&self, node_id: &NodeId, version: StateVersion, root: StateRoot) {
        if let Some(peer) = self.peers.write().get_mut(node_id) {
            peer.update_state(version, root);
        }
    }
    
    /// Get all peers
    pub fn all_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().values().cloned().collect()
    }
    
    /// Get validator peers
    pub fn validators(&self) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| p.is_validator)
            .cloned()
            .collect()
    }
    
    /// Get peers with the latest state
    pub fn peers_at_version(&self, version: StateVersion) -> Vec<PeerInfo> {
        self.peers
            .read()
            .values()
            .filter(|p| p.state_version >= version)
            .cloned()
            .collect()
    }
    
    /// Remove stale peers
    pub fn remove_stale_peers(&self, timeout_ms: u64) -> Vec<NodeId> {
        let mut peers = self.peers.write();
        let stale: Vec<NodeId> = peers
            .iter()
            .filter(|(_, p)| p.is_stale(timeout_ms))
            .map(|(id, _)| *id)
            .collect();
        
        for id in &stale {
            peers.remove(id);
        }
        
        stale
    }
    
    /// Number of connected peers
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }
    
    /// Number of validator peers
    pub fn validator_count(&self) -> usize {
        self.peers.read().values().filter(|p| p.is_validator).count()
    }
}

/// Shared peer manager
pub type SharedPeerManager = Arc<PeerManager>;

/// Create a shared peer manager
pub fn create_peer_manager(max_peers: usize) -> SharedPeerManager {
    Arc::new(PeerManager::new(max_peers))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_peer_manager() {
        let manager = PeerManager::new(10);
        
        let peer1 = PeerInfo::new(NodeId::from_bytes([1u8; 32]), true);
        let peer2 = PeerInfo::new(NodeId::from_bytes([2u8; 32]), false);
        
        assert!(manager.add_peer(peer1.clone()));
        assert!(manager.add_peer(peer2));
        
        assert_eq!(manager.peer_count(), 2);
        assert_eq!(manager.validator_count(), 1);
        
        manager.remove_peer(&peer1.node_id);
        assert_eq!(manager.peer_count(), 1);
    }
    
    #[test]
    fn test_max_peers() {
        let manager = PeerManager::new(2);
        
        assert!(manager.add_peer(PeerInfo::new(NodeId::from_bytes([1u8; 32]), false)));
        assert!(manager.add_peer(PeerInfo::new(NodeId::from_bytes([2u8; 32]), false)));
        assert!(!manager.add_peer(PeerInfo::new(NodeId::from_bytes([3u8; 32]), false)));
    }
}
