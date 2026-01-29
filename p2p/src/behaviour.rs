//! Network behaviour for libp2p

use libp2p::{
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    mdns,
    swarm::NetworkBehaviour,
    identity::Keypair,
};
use std::time::Duration;

/// Topic names for gossipsub
pub const TOPIC_TRANSACTIONS: &str = "rainsonet/transactions/1";
pub const TOPIC_PROPOSALS: &str = "rainsonet/proposals/1";
pub const TOPIC_VOTES: &str = "rainsonet/votes/1";
pub const TOPIC_SYNC: &str = "rainsonet/sync/1";

/// Combined network behaviour
#[derive(NetworkBehaviour)]
pub struct RainsonetBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}

impl RainsonetBehaviour {
    pub fn new(keypair: &Keypair, enable_mdns: bool) -> Result<Self, Box<dyn std::error::Error>> {
        // Configure gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(|message| {
                // Use hash of data as message ID for deduplication
                let hash = rainsonet_crypto::hashing::hash(&message.data);
                gossipsub::MessageId::from(hash.to_hex())
            })
            .build()
            .map_err(|e| format!("Failed to build gossipsub config: {}", e))?;
        
        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| format!("Failed to create gossipsub: {}", e))?;
        
        // Configure mDNS
        let mdns = if enable_mdns {
            mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                keypair.public().to_peer_id(),
            )?
        } else {
            mdns::tokio::Behaviour::new(
                mdns::Config {
                    enable_ipv6: false,
                    ..Default::default()
                },
                keypair.public().to_peer_id(),
            )?
        };
        
        Ok(Self { gossipsub, mdns })
    }
    
    /// Subscribe to all RAINSONET topics
    pub fn subscribe_all(&mut self) -> Result<(), gossipsub::SubscriptionError> {
        self.gossipsub.subscribe(&IdentTopic::new(TOPIC_TRANSACTIONS))?;
        self.gossipsub.subscribe(&IdentTopic::new(TOPIC_PROPOSALS))?;
        self.gossipsub.subscribe(&IdentTopic::new(TOPIC_VOTES))?;
        self.gossipsub.subscribe(&IdentTopic::new(TOPIC_SYNC))?;
        Ok(())
    }
    
    /// Publish a message to a topic
    pub fn publish(
        &mut self,
        topic: &str,
        data: Vec<u8>,
    ) -> Result<gossipsub::MessageId, gossipsub::PublishError> {
        self.gossipsub.publish(IdentTopic::new(topic), data)
    }
}

/// Get topic for message type
pub fn topic_for_message(message_type: &str) -> &'static str {
    match message_type {
        "transaction" => TOPIC_TRANSACTIONS,
        "proposal" => TOPIC_PROPOSALS,
        "vote" => TOPIC_VOTES,
        "sync_request" | "sync_response" => TOPIC_SYNC,
        _ => TOPIC_TRANSACTIONS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_topic_mapping() {
        assert_eq!(topic_for_message("transaction"), TOPIC_TRANSACTIONS);
        assert_eq!(topic_for_message("proposal"), TOPIC_PROPOSALS);
        assert_eq!(topic_for_message("vote"), TOPIC_VOTES);
    }
}
