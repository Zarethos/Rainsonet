//! Main network implementation

use crate::behaviour::{RainsonetBehaviour, RainsonetBehaviourEvent, TOPIC_PROPOSALS, TOPIC_TRANSACTIONS, TOPIC_VOTES};
use crate::message::{Message, TransactionMessage};
use crate::peer::{create_peer_manager, PeerInfo, SharedPeerManager};
use anyhow::Result;
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic},
    identity::Keypair,
    mdns,
    multiaddr::Protocol,
    swarm::{SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use rainsonet_core::{Hash, NetworkConfig, NodeId, RainsonetResult, StateRoot, StateVersion};
use rainsonet_crypto::keys::KeyPair as RainsonetKeyPair;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Network event for consumers
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Peer connected
    PeerConnected(NodeId),
    /// Peer disconnected
    PeerDisconnected(NodeId),
    /// Transaction received
    TransactionReceived(Hash, Vec<u8>),
    /// Proposal received
    ProposalReceived(Vec<u8>),
    /// Vote received
    VoteReceived(Vec<u8>),
    /// Sync request received
    SyncRequestReceived(Vec<u8>),
}

/// Network service for RAINSONET
pub struct NetworkService {
    swarm: Swarm<RainsonetBehaviour>,
    peer_manager: SharedPeerManager,
    node_id: NodeId,
    event_tx: mpsc::Sender<NetworkEvent>,
}

impl NetworkService {
    /// Create a new network service
    pub async fn new(
        keypair: &RainsonetKeyPair,
        config: &NetworkConfig,
        event_tx: mpsc::Sender<NetworkEvent>,
    ) -> Result<Self> {
        // Convert our keypair to libp2p keypair
        let libp2p_keypair = Keypair::ed25519_from_bytes(keypair.secret_bytes().to_vec())?;
        
        // Create swarm
        let behaviour = RainsonetBehaviour::new(&libp2p_keypair, config.enable_mdns)?;
        
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(libp2p_keypair)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_| behaviour)?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(std::time::Duration::from_secs(60))
            })
            .build();
        
        // Subscribe to topics
        swarm.behaviour_mut().subscribe_all()?;
        
        // Parse and listen on address
        let listen_addr: Multiaddr = config.listen_addr.parse()?;
        swarm.listen_on(listen_addr)?;
        
        let node_id = keypair.node_id();
        let peer_manager = create_peer_manager(config.max_peers);
        
        info!("Network service created for node {}", node_id);
        
        Ok(Self {
            swarm,
            peer_manager,
            node_id,
            event_tx,
        })
    }
    
    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
    
    /// Get the peer manager
    pub fn peer_manager(&self) -> SharedPeerManager {
        self.peer_manager.clone()
    }
    
    /// Connect to bootstrap nodes
    pub async fn connect_bootstrap(&mut self, nodes: &[String]) -> Result<()> {
        for addr_str in nodes {
            match addr_str.parse::<Multiaddr>() {
                Ok(addr) => {
                    info!("Connecting to bootstrap node: {}", addr);
                    if let Err(e) = self.swarm.dial(addr.clone()) {
                        warn!("Failed to dial {}: {}", addr, e);
                    }
                }
                Err(e) => {
                    warn!("Invalid bootstrap address {}: {}", addr_str, e);
                }
            }
        }
        Ok(())
    }
    
    /// Broadcast a transaction
    pub fn broadcast_transaction(&mut self, tx_id: Hash, tx_data: Vec<u8>) -> Result<()> {
        let msg = Message::Transaction(TransactionMessage::new(tx_id, tx_data));
        let data = msg.to_bytes();
        
        self.swarm
            .behaviour_mut()
            .publish(TOPIC_TRANSACTIONS, data)?;
        
        debug!("Broadcast transaction {}", tx_id);
        Ok(())
    }
    
    /// Broadcast a proposal
    pub fn broadcast_proposal(&mut self, proposal_data: Vec<u8>) -> Result<()> {
        let msg = Message::Proposal(bincode::deserialize(&proposal_data)?);
        let data = msg.to_bytes();
        
        self.swarm.behaviour_mut().publish(TOPIC_PROPOSALS, data)?;
        
        debug!("Broadcast proposal");
        Ok(())
    }
    
    /// Broadcast a vote
    pub fn broadcast_vote(&mut self, vote_data: Vec<u8>) -> Result<()> {
        let msg = Message::Vote(bincode::deserialize(&vote_data)?);
        let data = msg.to_bytes();
        
        self.swarm.behaviour_mut().publish(TOPIC_VOTES, data)?;
        
        debug!("Broadcast vote");
        Ok(())
    }
    
    /// Run the network event loop
    pub async fn run(&mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(event) => {
                    self.handle_behaviour_event(event).await;
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    self.handle_peer_connected(peer_id).await;
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    self.handle_peer_disconnected(peer_id).await;
                }
                _ => {}
            }
        }
    }
    
    async fn handle_behaviour_event(&mut self, event: RainsonetBehaviourEvent) {
        match event {
            RainsonetBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                message,
                propagation_source,
                ..
            }) => {
                self.handle_gossip_message(message, propagation_source).await;
            }
            RainsonetBehaviourEvent::Mdns(mdns::Event::Discovered(peers)) => {
                for (peer_id, addr) in peers {
                    info!("mDNS discovered peer: {} at {}", peer_id, addr);
                    if let Err(e) = self.swarm.dial(addr) {
                        warn!("Failed to dial discovered peer: {}", e);
                    }
                }
            }
            RainsonetBehaviourEvent::Mdns(mdns::Event::Expired(peers)) => {
                for (peer_id, _) in peers {
                    debug!("mDNS peer expired: {}", peer_id);
                }
            }
            _ => {}
        }
    }
    
    async fn handle_gossip_message(&self, message: gossipsub::Message, source: PeerId) {
        let topic = message.topic.as_str();
        
        if let Some(msg) = Message::from_bytes(&message.data) {
            match msg {
                Message::Transaction(tx_msg) => {
                    let _ = self.event_tx.send(NetworkEvent::TransactionReceived(
                        tx_msg.tx_id,
                        tx_msg.tx_data,
                    )).await;
                }
                Message::Proposal(proposal_msg) => {
                    let data = bincode::serialize(&proposal_msg).unwrap_or_default();
                    let _ = self.event_tx.send(NetworkEvent::ProposalReceived(data)).await;
                }
                Message::Vote(vote_msg) => {
                    let data = bincode::serialize(&vote_msg).unwrap_or_default();
                    let _ = self.event_tx.send(NetworkEvent::VoteReceived(data)).await;
                }
                Message::SyncRequest(sync_msg) => {
                    let data = bincode::serialize(&sync_msg).unwrap_or_default();
                    let _ = self.event_tx.send(NetworkEvent::SyncRequestReceived(data)).await;
                }
                _ => {}
            }
        }
    }
    
    async fn handle_peer_connected(&self, peer_id: PeerId) {
        // Convert PeerId to NodeId
        let peer_bytes = peer_id.to_bytes();
        let mut node_id_bytes = [0u8; 32];
        let len = peer_bytes.len().min(32);
        node_id_bytes[..len].copy_from_slice(&peer_bytes[..len]);
        let node_id = NodeId::from_bytes(node_id_bytes);
        
        info!("Peer connected: {}", peer_id);
        
        let peer_info = PeerInfo::new(node_id, false);
        self.peer_manager.add_peer(peer_info);
        
        let _ = self.event_tx.send(NetworkEvent::PeerConnected(node_id)).await;
    }
    
    async fn handle_peer_disconnected(&self, peer_id: PeerId) {
        let peer_bytes = peer_id.to_bytes();
        let mut node_id_bytes = [0u8; 32];
        let len = peer_bytes.len().min(32);
        node_id_bytes[..len].copy_from_slice(&peer_bytes[..len]);
        let node_id = NodeId::from_bytes(node_id_bytes);
        
        info!("Peer disconnected: {}", peer_id);
        
        self.peer_manager.remove_peer(&node_id);
        
        let _ = self.event_tx.send(NetworkEvent::PeerDisconnected(node_id)).await;
    }
}

/// Create network event channel
pub fn create_network_channel() -> (mpsc::Sender<NetworkEvent>, mpsc::Receiver<NetworkEvent>) {
    mpsc::channel(1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_event_channel() {
        let (tx, mut rx) = create_network_channel();
        
        // Channel should be created
        assert!(tx.is_closed() == false);
    }
}
