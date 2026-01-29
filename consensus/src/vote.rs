//! Vote handling for consensus

use rainsonet_core::{Hash, NodeId, Signature, StateRoot, StateVersion, Timestamp};
use rainsonet_crypto::hashing::hash_multiple;
use serde::{Deserialize, Serialize};

/// Vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Proposal being voted on
    pub proposal_id: Hash,
    /// Voting validator
    pub voter: NodeId,
    /// Approval or rejection
    pub approve: bool,
    /// Voter's current state version
    pub state_version: StateVersion,
    /// Voter's current state root
    pub state_root: StateRoot,
    /// Voter's signature
    pub signature: Signature,
    /// Vote timestamp
    pub timestamp: Timestamp,
}

impl Vote {
    /// Create a new vote
    pub fn new(
        proposal_id: Hash,
        voter: NodeId,
        approve: bool,
        state_version: StateVersion,
        state_root: StateRoot,
        sign_fn: impl FnOnce(&[u8]) -> Signature,
    ) -> Self {
        let timestamp = Timestamp::now();
        
        let sign_msg = Self::signing_message(
            &proposal_id,
            &voter,
            approve,
            state_version,
            &state_root,
            &timestamp,
        );
        
        let signature = sign_fn(&sign_msg);
        
        Self {
            proposal_id,
            voter,
            approve,
            state_version,
            state_root,
            signature,
            timestamp,
        }
    }
    
    /// Create signing message
    fn signing_message(
        proposal_id: &Hash,
        voter: &NodeId,
        approve: bool,
        state_version: StateVersion,
        state_root: &StateRoot,
        timestamp: &Timestamp,
    ) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"RAINSONET_VOTE:");
        msg.extend_from_slice(proposal_id.as_bytes());
        msg.extend_from_slice(voter.as_bytes());
        msg.push(if approve { 1 } else { 0 });
        msg.extend_from_slice(&state_version.0.to_le_bytes());
        msg.extend_from_slice(state_root.as_bytes());
        msg.extend_from_slice(&timestamp.0.to_le_bytes());
        msg
    }
    
    /// Get signing message for verification
    pub fn get_signing_message(&self) -> Vec<u8> {
        Self::signing_message(
            &self.proposal_id,
            &self.voter,
            self.approve,
            self.state_version,
            &self.state_root,
            &self.timestamp,
        )
    }
    
    /// Check if vote is expired
    pub fn is_expired(&self, timeout_ms: u64) -> bool {
        let now = Timestamp::now();
        now.as_millis() - self.timestamp.as_millis() > timeout_ms
    }
}

/// Vote collection for a proposal
#[derive(Debug, Default)]
pub struct VoteCollection {
    pub votes: Vec<Vote>,
    pub votes_for: usize,
    pub votes_against: usize,
}

impl VoteCollection {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a vote (returns false if duplicate)
    pub fn add(&mut self, vote: Vote) -> bool {
        // Check for duplicate
        if self.votes.iter().any(|v| v.voter == vote.voter) {
            return false;
        }
        
        if vote.approve {
            self.votes_for += 1;
        } else {
            self.votes_against += 1;
        }
        
        self.votes.push(vote);
        true
    }
    
    /// Check if consensus is reached
    pub fn has_consensus(&self, required: usize) -> bool {
        self.votes_for >= required
    }
    
    /// Check if rejected
    pub fn is_rejected(&self, total_validators: usize, required: usize) -> bool {
        self.votes_against > total_validators - required
    }
    
    /// Total votes
    pub fn total(&self) -> usize {
        self.votes.len()
    }
}

/// Finality certificate - proof of consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityCertificate {
    pub proposal_id: Hash,
    pub state_version: StateVersion,
    pub state_root: StateRoot,
    pub votes: Vec<Vote>,
    pub finalized_at: Timestamp,
}

impl FinalityCertificate {
    pub fn new(
        proposal_id: Hash,
        state_version: StateVersion,
        state_root: StateRoot,
        votes: Vec<Vote>,
    ) -> Self {
        Self {
            proposal_id,
            state_version,
            state_root,
            votes,
            finalized_at: Timestamp::now(),
        }
    }
    
    /// Verify the certificate
    pub fn verify(&self, required_votes: usize) -> bool {
        let approvals = self.votes.iter().filter(|v| v.approve).count();
        approvals >= required_votes
    }
    
    /// Get voter node IDs
    pub fn voters(&self) -> Vec<NodeId> {
        self.votes.iter().map(|v| v.voter).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    use rainsonet_crypto::signing::sign;
    
    #[test]
    fn test_vote_creation() {
        let kp = KeyPair::generate();
        let node_id = kp.node_id();
        let proposal_id = Hash::from_bytes([1u8; 32]);
        
        let vote = Vote::new(
            proposal_id,
            node_id,
            true,
            StateVersion::new(1),
            Hash::ZERO,
            |msg| sign(&kp, msg),
        );
        
        assert!(vote.approve);
        assert_eq!(vote.voter, node_id);
    }
    
    #[test]
    fn test_vote_collection() {
        let mut collection = VoteCollection::new();
        
        for i in 0..3 {
            let kp = KeyPair::generate();
            let vote = Vote::new(
                Hash::from_bytes([1u8; 32]),
                kp.node_id(),
                i < 2, // First 2 approve, last rejects
                StateVersion::new(1),
                Hash::ZERO,
                |msg| sign(&kp, msg),
            );
            assert!(collection.add(vote));
        }
        
        assert_eq!(collection.votes_for, 2);
        assert_eq!(collection.votes_against, 1);
        assert!(collection.has_consensus(2)); // 2/3 majority
    }
}
