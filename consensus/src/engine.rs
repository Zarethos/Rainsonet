//! Main consensus engine implementation

use crate::proposal::{Proposal, ProposalStatus, ProposalStore};
use crate::validator::{LocalValidator, SharedValidatorSet, ValidatorSet};
use crate::vote::{FinalityCertificate, Vote, VoteCollection};
use async_trait::async_trait;
use parking_lot::RwLock;
use rainsonet_core::{
    ConsensusConfig, ConsensusEngine as ConsensusEngineTrait, Hash, NodeId,
    RainsonetError, RainsonetResult, StateChange, StateRoot, StateVersion, Timestamp,
};
use rainsonet_crypto::keys::KeyPair;
use rainsonet_crypto::signing::sign;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Events emitted by the consensus engine
#[derive(Debug, Clone)]
pub enum ConsensusEvent {
    /// New proposal created
    ProposalCreated(Hash),
    /// Received a proposal
    ProposalReceived(Hash),
    /// Vote cast
    VoteCast(Hash, NodeId, bool),
    /// State finalized
    StateFinalized(StateVersion, StateRoot, FinalityCertificate),
    /// Proposal rejected
    ProposalRejected(Hash),
    /// Proposal expired
    ProposalExpired(Hash),
}

/// Consensus engine for RAINSONET
pub struct RainsonetConsensus {
    config: ConsensusConfig,
    validator_set: SharedValidatorSet,
    local_validator: Option<LocalValidator>,
    proposal_store: ProposalStore,
    vote_collections: RwLock<HashMap<Hash, VoteCollection>>,
    finalized_version: RwLock<StateVersion>,
    finalized_root: RwLock<StateRoot>,
    certificates: RwLock<Vec<FinalityCertificate>>,
    event_tx: Option<mpsc::Sender<ConsensusEvent>>,
}

impl RainsonetConsensus {
    /// Create a new consensus engine
    pub fn new(
        config: ConsensusConfig,
        validator_set: SharedValidatorSet,
        local_keypair: Option<KeyPair>,
    ) -> Self {
        let local_validator = if config.is_validator {
            local_keypair.map(LocalValidator::new)
        } else {
            None
        };
        
        Self {
            config,
            validator_set,
            local_validator,
            proposal_store: ProposalStore::new(),
            vote_collections: RwLock::new(HashMap::new()),
            finalized_version: RwLock::new(StateVersion::new(0)),
            finalized_root: RwLock::new(Hash::ZERO),
            certificates: RwLock::new(Vec::new()),
            event_tx: None,
        }
    }
    
    /// Set event channel
    pub fn set_event_channel(&mut self, tx: mpsc::Sender<ConsensusEvent>) {
        self.event_tx = Some(tx);
    }
    
    /// Check if this node is a validator
    pub fn is_validator(&self) -> bool {
        self.local_validator.is_some()
    }
    
    /// Get local node ID
    pub fn node_id(&self) -> Option<NodeId> {
        self.local_validator.as_ref().map(|v| v.node_id())
    }
    
    /// Create a proposal for state changes
    pub fn create_proposal(
        &self,
        previous_root: StateRoot,
        new_root: StateRoot,
        tx_ids: Vec<Hash>,
        changes: Vec<StateChange>,
    ) -> RainsonetResult<Proposal> {
        let local = self
            .local_validator
            .as_ref()
            .ok_or(RainsonetError::NotAValidator)?;
        
        let next_version = self.finalized_version.read().next();
        
        let proposal = Proposal::new(
            local.node_id(),
            next_version,
            previous_root,
            new_root,
            tx_ids,
            &changes,
            |msg| local.sign(msg),
        );
        
        // Store the proposal
        self.proposal_store.add(proposal.clone(), changes);
        self.vote_collections
            .write()
            .insert(proposal.id, VoteCollection::new());
        
        info!(
            "Created proposal {} for version {}",
            proposal.id, next_version
        );
        
        self.emit_event(ConsensusEvent::ProposalCreated(proposal.id));
        
        Ok(proposal)
    }
    
    /// Receive and validate a proposal
    pub fn receive_proposal(&self, proposal: Proposal, changes: Vec<StateChange>) -> RainsonetResult<()> {
        // Validate proposer is a validator
        if !self.validator_set.is_validator(&proposal.proposer) {
            return Err(RainsonetError::NotAValidator);
        }
        
        // Validate signature
        let sign_msg = proposal.get_signing_message();
        self.validator_set
            .verify_signature(&proposal.proposer, &sign_msg, &proposal.signature)?;
        
        // Validate version
        let expected_version = self.finalized_version.read().next();
        if proposal.state_version != expected_version {
            return Err(RainsonetError::StateVersionMismatch {
                expected: expected_version.0,
                got: proposal.state_version.0,
            });
        }
        
        // Store proposal
        self.proposal_store.add(proposal.clone(), changes);
        self.vote_collections
            .write()
            .insert(proposal.id, VoteCollection::new());
        
        info!("Received proposal {} from {}", proposal.id, proposal.proposer);
        
        self.emit_event(ConsensusEvent::ProposalReceived(proposal.id));
        
        // Auto-vote if we're a validator
        if self.is_validator() {
            self.vote_on_proposal(&proposal.id, true)?;
        }
        
        Ok(())
    }
    
    /// Cast a vote on a proposal
    pub fn vote_on_proposal(&self, proposal_id: &Hash, approve: bool) -> RainsonetResult<Vote> {
        let local = self
            .local_validator
            .as_ref()
            .ok_or(RainsonetError::NotAValidator)?;
        
        let proposal = self
            .proposal_store
            .get(proposal_id)
            .ok_or(RainsonetError::ProposalRejected("Proposal not found".into()))?;
        
        let vote = Vote::new(
            *proposal_id,
            local.node_id(),
            approve,
            *self.finalized_version.read(),
            *self.finalized_root.read(),
            |msg| local.sign(msg),
        );
        
        // Add our own vote
        self.receive_vote(vote.clone())?;
        
        Ok(vote)
    }
    
    /// Receive and process a vote
    pub fn receive_vote(&self, vote: Vote) -> RainsonetResult<()> {
        // Validate voter is a validator
        if !self.validator_set.is_validator(&vote.voter) {
            return Err(RainsonetError::NotAValidator);
        }
        
        // Validate signature
        let sign_msg = vote.get_signing_message();
        self.validator_set
            .verify_signature(&vote.voter, &sign_msg, &vote.signature)?;
        
        // Add to collection
        let mut collections = self.vote_collections.write();
        if let Some(collection) = collections.get_mut(&vote.proposal_id) {
            if !collection.add(vote.clone()) {
                debug!("Duplicate vote from {}", vote.voter);
                return Ok(());
            }
            
            info!(
                "Vote received for proposal {}: {} from {}",
                vote.proposal_id,
                if vote.approve { "approve" } else { "reject" },
                vote.voter
            );
            
            self.emit_event(ConsensusEvent::VoteCast(
                vote.proposal_id,
                vote.voter,
                vote.approve,
            ));
            
            // Check for consensus
            let required = self.validator_set.required_votes();
            let total = self.validator_set.active_count();
            
            if collection.has_consensus(required) {
                drop(collections);
                self.finalize_proposal(&vote.proposal_id)?;
            } else if collection.is_rejected(total, required) {
                self.proposal_store
                    .add_vote(&vote.proposal_id, vote.voter, vote.approve);
                self.emit_event(ConsensusEvent::ProposalRejected(vote.proposal_id));
            }
        }
        
        Ok(())
    }
    
    /// Finalize a proposal that reached consensus
    fn finalize_proposal(&self, proposal_id: &Hash) -> RainsonetResult<()> {
        let proposal = self
            .proposal_store
            .get(proposal_id)
            .ok_or(RainsonetError::ProposalRejected("Proposal not found".into()))?;
        
        let votes: Vec<Vote> = self
            .vote_collections
            .read()
            .get(proposal_id)
            .map(|c| c.votes.clone())
            .unwrap_or_default();
        
        // Create finality certificate
        let certificate = FinalityCertificate::new(
            *proposal_id,
            proposal.state_version,
            proposal.new_root,
            votes,
        );
        
        // Update finalized state
        *self.finalized_version.write() = proposal.state_version;
        *self.finalized_root.write() = proposal.new_root;
        self.certificates.write().push(certificate.clone());
        
        info!(
            "State finalized: version {} root {}",
            proposal.state_version, proposal.new_root
        );
        
        self.emit_event(ConsensusEvent::StateFinalized(
            proposal.state_version,
            proposal.new_root,
            certificate,
        ));
        
        Ok(())
    }
    
    /// Get the latest finalized version
    pub fn latest_finalized_version(&self) -> StateVersion {
        *self.finalized_version.read()
    }
    
    /// Get the latest finalized root
    pub fn latest_finalized_root(&self) -> StateRoot {
        *self.finalized_root.read()
    }
    
    /// Get a finality certificate
    pub fn get_certificate(&self, version: StateVersion) -> Option<FinalityCertificate> {
        self.certificates
            .read()
            .iter()
            .find(|c| c.state_version == version)
            .cloned()
    }
    
    /// Get state changes for an approved proposal
    pub fn get_finalized_changes(&self, proposal_id: &Hash) -> Option<Vec<StateChange>> {
        self.proposal_store.get_approved_changes(proposal_id)
    }
    
    fn emit_event(&self, event: ConsensusEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.try_send(event);
        }
    }
    
    /// Cleanup old proposals
    pub fn cleanup(&self) {
        let finalized = *self.finalized_version.read();
        if finalized.0 > 10 {
            self.proposal_store
                .cleanup(StateVersion::new(finalized.0 - 10));
        }
    }
}

#[async_trait]
impl ConsensusEngineTrait for RainsonetConsensus {
    async fn propose(&self, changes: Vec<StateChange>) -> RainsonetResult<StateVersion> {
        let current_root = *self.finalized_root.read();
        // Note: In production, we'd compute the new root from changes
        let new_root = rainsonet_crypto::hashing::hash(&bincode::serialize(&changes)?);
        
        let proposal = self.create_proposal(current_root, new_root, vec![], changes)?;
        
        // Wait for finalization (simplified)
        Ok(proposal.state_version)
    }
    
    async fn vote(&self, vote: rainsonet_core::Vote) -> RainsonetResult<()> {
        let v = Vote::new(
            vote.state_root, // Using state_root as proposal_id for compatibility
            vote.voter,
            true,
            vote.state_version,
            vote.state_root,
            |_| vote.signature,
        );
        self.receive_vote(v)
    }
    
    async fn is_finalized(&self, version: StateVersion) -> bool {
        self.finalized_version.read().0 >= version.0
    }
    
    async fn latest_finalized(&self) -> StateVersion {
        self.latest_finalized_version()
    }
}

/// Shared consensus engine
pub type SharedConsensus = Arc<RainsonetConsensus>;

/// Create consensus event channel
pub fn create_consensus_channel() -> (mpsc::Sender<ConsensusEvent>, mpsc::Receiver<ConsensusEvent>) {
    mpsc::channel(100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::ValidatorInfo;
    
    fn setup_validators(count: usize) -> (Vec<KeyPair>, SharedValidatorSet) {
        let keypairs: Vec<KeyPair> = (0..count).map(|_| KeyPair::generate()).collect();
        
        let validators: Vec<ValidatorInfo> = keypairs
            .iter()
            .map(|kp| ValidatorInfo::new(kp.node_id(), kp.public_key(), 1000))
            .collect();
        
        let set = Arc::new(ValidatorSet::with_validators(validators));
        
        (keypairs, set)
    }
    
    #[test]
    fn test_proposal_creation() {
        let (keypairs, validator_set) = setup_validators(3);
        
        let config = ConsensusConfig {
            is_validator: true,
            ..Default::default()
        };
        
        let consensus = RainsonetConsensus::new(config, validator_set, Some(keypairs[0].clone()));
        
        let changes = vec![StateChange::Set {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        }];
        
        let proposal = consensus
            .create_proposal(Hash::ZERO, Hash::from_bytes([1u8; 32]), vec![], changes)
            .unwrap();
        
        assert_eq!(proposal.state_version.0, 1);
    }
    
    #[test]
    fn test_consensus_flow() {
        let (keypairs, validator_set) = setup_validators(3);
        
        // Create consensus engines for each validator
        let engines: Vec<RainsonetConsensus> = keypairs
            .iter()
            .map(|kp| {
                let config = ConsensusConfig {
                    is_validator: true,
                    ..Default::default()
                };
                RainsonetConsensus::new(config, validator_set.clone(), Some(kp.clone()))
            })
            .collect();
        
        // Validator 0 creates proposal
        let changes = vec![StateChange::Set {
            key: b"test".to_vec(),
            value: b"value".to_vec(),
        }];
        
        let proposal = engines[0]
            .create_proposal(Hash::ZERO, Hash::from_bytes([1u8; 32]), vec![], changes.clone())
            .unwrap();
        
        // Other validators receive and vote
        for engine in &engines[1..] {
            engine.receive_proposal(proposal.clone(), changes.clone()).unwrap();
        }
        
        // Manually process votes (simulating network)
        let vote1 = engines[1].vote_on_proposal(&proposal.id, true).unwrap();
        engines[0].receive_vote(vote1).unwrap();
        
        let vote2 = engines[2].vote_on_proposal(&proposal.id, true).unwrap();
        engines[0].receive_vote(vote2).unwrap();
        
        // Check finalization
        assert_eq!(engines[0].latest_finalized_version().0, 1);
    }
}
