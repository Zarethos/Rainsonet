//! Proposal management for consensus

use rainsonet_core::{Hash, NodeId, Signature, StateChange, StateRoot, StateVersion, Timestamp};
use rainsonet_crypto::hashing::hash_multiple;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

/// State update proposal from a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique proposal ID
    pub id: Hash,
    /// Proposing validator
    pub proposer: NodeId,
    /// Target state version
    pub state_version: StateVersion,
    /// Previous state root
    pub previous_root: StateRoot,
    /// New state root after changes
    pub new_root: StateRoot,
    /// Transaction IDs included
    pub tx_ids: Vec<Hash>,
    /// Hash of state changes
    pub changes_hash: Hash,
    /// Proposer's signature
    pub signature: Signature,
    /// Creation timestamp
    pub timestamp: Timestamp,
}

impl Proposal {
    /// Create a new proposal
    pub fn new(
        proposer: NodeId,
        state_version: StateVersion,
        previous_root: StateRoot,
        new_root: StateRoot,
        tx_ids: Vec<Hash>,
        changes: &[StateChange],
        sign_fn: impl FnOnce(&[u8]) -> Signature,
    ) -> Self {
        let changes_hash = Self::compute_changes_hash(changes);
        let timestamp = Timestamp::now();
        
        // Compute proposal ID
        let id_data = [
            proposer.as_bytes().as_slice(),
            &state_version.0.to_le_bytes(),
            previous_root.as_bytes(),
            new_root.as_bytes(),
            &timestamp.0.to_le_bytes(),
        ];
        let id = hash_multiple(&id_data);
        
        // Create signing message
        let sign_msg = Self::signing_message(
            &id,
            &proposer,
            state_version,
            &previous_root,
            &new_root,
            &changes_hash,
        );
        
        let signature = sign_fn(&sign_msg);
        
        Self {
            id,
            proposer,
            state_version,
            previous_root,
            new_root,
            tx_ids,
            changes_hash,
            signature,
            timestamp,
        }
    }
    
    /// Compute hash of state changes
    fn compute_changes_hash(changes: &[StateChange]) -> Hash {
        let serialized = bincode::serialize(changes).unwrap_or_default();
        rainsonet_crypto::hashing::hash(&serialized)
    }
    
    /// Create signing message
    fn signing_message(
        id: &Hash,
        proposer: &NodeId,
        state_version: StateVersion,
        previous_root: &StateRoot,
        new_root: &StateRoot,
        changes_hash: &Hash,
    ) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"RAINSONET_PROPOSAL:");
        msg.extend_from_slice(id.as_bytes());
        msg.extend_from_slice(proposer.as_bytes());
        msg.extend_from_slice(&state_version.0.to_le_bytes());
        msg.extend_from_slice(previous_root.as_bytes());
        msg.extend_from_slice(new_root.as_bytes());
        msg.extend_from_slice(changes_hash.as_bytes());
        msg
    }
    
    /// Get the signing message for verification
    pub fn get_signing_message(&self) -> Vec<u8> {
        Self::signing_message(
            &self.id,
            &self.proposer,
            self.state_version,
            &self.previous_root,
            &self.new_root,
            &self.changes_hash,
        )
    }
    
    /// Check if proposal is expired
    pub fn is_expired(&self, timeout_ms: u64) -> bool {
        let now = Timestamp::now();
        now.as_millis() - self.timestamp.as_millis() > timeout_ms
    }
}

/// Proposal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Waiting for votes
    Pending,
    /// Reached consensus - approved
    Approved,
    /// Rejected by validators
    Rejected,
    /// Timed out
    Expired,
}

/// Tracked proposal with votes
#[derive(Debug)]
pub struct TrackedProposal {
    pub proposal: Proposal,
    pub status: ProposalStatus,
    pub votes_for: usize,
    pub votes_against: usize,
    pub voters: HashMap<NodeId, bool>,
    pub state_changes: Vec<StateChange>,
}

impl TrackedProposal {
    pub fn new(proposal: Proposal, state_changes: Vec<StateChange>) -> Self {
        Self {
            proposal,
            status: ProposalStatus::Pending,
            votes_for: 0,
            votes_against: 0,
            voters: HashMap::new(),
            state_changes,
        }
    }
    
    /// Add a vote
    pub fn add_vote(&mut self, voter: NodeId, approve: bool) -> bool {
        if self.voters.contains_key(&voter) {
            return false; // Already voted
        }
        
        self.voters.insert(voter, approve);
        if approve {
            self.votes_for += 1;
        } else {
            self.votes_against += 1;
        }
        
        true
    }
    
    /// Check and update status
    pub fn check_consensus(&mut self, required_votes: usize, total_validators: usize) {
        if self.votes_for >= required_votes {
            self.status = ProposalStatus::Approved;
        } else if self.votes_against > total_validators - required_votes {
            self.status = ProposalStatus::Rejected;
        }
    }
    
    /// Mark as expired
    pub fn expire(&mut self) {
        if self.status == ProposalStatus::Pending {
            self.status = ProposalStatus::Expired;
        }
    }
}

/// Proposal store
pub struct ProposalStore {
    proposals: RwLock<HashMap<Hash, TrackedProposal>>,
    by_version: RwLock<HashMap<StateVersion, Hash>>,
}

impl ProposalStore {
    pub fn new() -> Self {
        Self {
            proposals: RwLock::new(HashMap::new()),
            by_version: RwLock::new(HashMap::new()),
        }
    }
    
    /// Add a proposal
    pub fn add(&self, proposal: Proposal, changes: Vec<StateChange>) {
        let id = proposal.id;
        let version = proposal.state_version;
        
        self.proposals
            .write()
            .insert(id, TrackedProposal::new(proposal, changes));
        self.by_version.write().insert(version, id);
    }
    
    /// Get a proposal
    pub fn get(&self, id: &Hash) -> Option<Proposal> {
        self.proposals.read().get(id).map(|tp| tp.proposal.clone())
    }
    
    /// Get proposal status
    pub fn status(&self, id: &Hash) -> Option<ProposalStatus> {
        self.proposals.read().get(id).map(|tp| tp.status)
    }
    
    /// Add vote to proposal
    pub fn add_vote(&self, proposal_id: &Hash, voter: NodeId, approve: bool) -> bool {
        if let Some(tp) = self.proposals.write().get_mut(proposal_id) {
            tp.add_vote(voter, approve)
        } else {
            false
        }
    }
    
    /// Check consensus for proposal
    pub fn check_consensus(&self, proposal_id: &Hash, required_votes: usize, total_validators: usize) {
        if let Some(tp) = self.proposals.write().get_mut(proposal_id) {
            tp.check_consensus(required_votes, total_validators);
        }
    }
    
    /// Get state changes for approved proposal
    pub fn get_approved_changes(&self, proposal_id: &Hash) -> Option<Vec<StateChange>> {
        let proposals = self.proposals.read();
        proposals.get(proposal_id).and_then(|tp| {
            if tp.status == ProposalStatus::Approved {
                Some(tp.state_changes.clone())
            } else {
                None
            }
        })
    }
    
    /// Remove old proposals
    pub fn cleanup(&self, before_version: StateVersion) {
        let mut by_version = self.by_version.write();
        let mut proposals = self.proposals.write();
        
        let old_versions: Vec<StateVersion> = by_version
            .keys()
            .filter(|v| v.0 < before_version.0)
            .copied()
            .collect();
        
        for version in old_versions {
            if let Some(id) = by_version.remove(&version) {
                proposals.remove(&id);
            }
        }
    }
}

impl Default for ProposalStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    use rainsonet_crypto::signing::sign;
    
    #[test]
    fn test_proposal_creation() {
        let kp = KeyPair::generate();
        let node_id = kp.node_id();
        
        let proposal = Proposal::new(
            node_id,
            StateVersion::new(1),
            Hash::ZERO,
            Hash::from_bytes([1u8; 32]),
            vec![],
            &[],
            |msg| sign(&kp, msg),
        );
        
        assert_eq!(proposal.proposer, node_id);
        assert_eq!(proposal.state_version.0, 1);
    }
    
    #[test]
    fn test_tracked_proposal_voting() {
        let kp = KeyPair::generate();
        let node_id = kp.node_id();
        
        let proposal = Proposal::new(
            node_id,
            StateVersion::new(1),
            Hash::ZERO,
            Hash::from_bytes([1u8; 32]),
            vec![],
            &[],
            |msg| sign(&kp, msg),
        );
        
        let mut tracked = TrackedProposal::new(proposal, vec![]);
        
        // Add votes
        let voter1 = NodeId::from_bytes([1u8; 32]);
        let voter2 = NodeId::from_bytes([2u8; 32]);
        let voter3 = NodeId::from_bytes([3u8; 32]);
        
        assert!(tracked.add_vote(voter1, true));
        assert!(tracked.add_vote(voter2, true));
        assert!(tracked.add_vote(voter3, false));
        
        // Can't vote twice
        assert!(!tracked.add_vote(voter1, false));
        
        assert_eq!(tracked.votes_for, 2);
        assert_eq!(tracked.votes_against, 1);
        
        // Check consensus (2/3 of 3 = 2)
        tracked.check_consensus(2, 3);
        assert_eq!(tracked.status, ProposalStatus::Approved);
    }
}
