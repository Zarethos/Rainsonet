//! Validator management

use dashmap::DashMap;
use parking_lot::RwLock;
use rainsonet_core::{NodeId, PublicKey, RainsonetError, RainsonetResult, Signature};
use rainsonet_crypto::signing::{sign, verify};
use rainsonet_crypto::keys::KeyPair;
use std::collections::HashSet;
use std::sync::Arc;

/// Validator information
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub node_id: NodeId,
    pub public_key: PublicKey,
    pub stake: u128,
    pub active: bool,
}

impl ValidatorInfo {
    pub fn new(node_id: NodeId, public_key: PublicKey, stake: u128) -> Self {
        Self {
            node_id,
            public_key,
            stake,
            active: true,
        }
    }
}

/// Validator set management
pub struct ValidatorSet {
    validators: DashMap<NodeId, ValidatorInfo>,
    active_count: RwLock<usize>,
}

impl ValidatorSet {
    pub fn new() -> Self {
        Self {
            validators: DashMap::new(),
            active_count: RwLock::new(0),
        }
    }
    
    /// Create with initial validators
    pub fn with_validators(validators: Vec<ValidatorInfo>) -> Self {
        let set = Self::new();
        let mut count = 0;
        for v in validators {
            if v.active {
                count += 1;
            }
            set.validators.insert(v.node_id, v);
        }
        *set.active_count.write() = count;
        set
    }
    
    /// Add a validator
    pub fn add_validator(&self, info: ValidatorInfo) {
        if info.active && !self.validators.contains_key(&info.node_id) {
            *self.active_count.write() += 1;
        }
        self.validators.insert(info.node_id, info);
    }
    
    /// Remove a validator
    pub fn remove_validator(&self, node_id: &NodeId) {
        if let Some((_, v)) = self.validators.remove(node_id) {
            if v.active {
                *self.active_count.write() -= 1;
            }
        }
    }
    
    /// Check if a node is a validator
    pub fn is_validator(&self, node_id: &NodeId) -> bool {
        self.validators
            .get(node_id)
            .map(|v| v.active)
            .unwrap_or(false)
    }
    
    /// Get validator info
    pub fn get_validator(&self, node_id: &NodeId) -> Option<ValidatorInfo> {
        self.validators.get(node_id).map(|v| v.clone())
    }
    
    /// Get validator public key
    pub fn get_public_key(&self, node_id: &NodeId) -> Option<PublicKey> {
        self.validators.get(node_id).map(|v| v.public_key)
    }
    
    /// Get all active validators
    pub fn active_validators(&self) -> Vec<ValidatorInfo> {
        self.validators
            .iter()
            .filter(|v| v.active)
            .map(|v| v.clone())
            .collect()
    }
    
    /// Get active validator count
    pub fn active_count(&self) -> usize {
        *self.active_count.read()
    }
    
    /// Calculate required votes for consensus (2/3 majority)
    pub fn required_votes(&self) -> usize {
        let count = self.active_count();
        (count * 2 / 3) + 1
    }
    
    /// Total stake of active validators
    pub fn total_stake(&self) -> u128 {
        self.validators
            .iter()
            .filter(|v| v.active)
            .map(|v| v.stake)
            .sum()
    }
    
    /// Verify a signature from a validator
    pub fn verify_signature(
        &self,
        node_id: &NodeId,
        message: &[u8],
        signature: &Signature,
    ) -> RainsonetResult<()> {
        let public_key = self
            .get_public_key(node_id)
            .ok_or(RainsonetError::NotAValidator)?;
        
        verify(&public_key, message, signature)
    }
}

impl Default for ValidatorSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Local validator identity
pub struct LocalValidator {
    keypair: KeyPair,
    node_id: NodeId,
}

impl LocalValidator {
    pub fn new(keypair: KeyPair) -> Self {
        let node_id = keypair.node_id();
        Self { keypair, node_id }
    }
    
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
    
    pub fn public_key(&self) -> PublicKey {
        self.keypair.public_key()
    }
    
    pub fn sign(&self, message: &[u8]) -> Signature {
        sign(&self.keypair, message)
    }
    
    pub fn to_validator_info(&self, stake: u128) -> ValidatorInfo {
        ValidatorInfo::new(self.node_id, self.public_key(), stake)
    }
}

/// Shared validator set
pub type SharedValidatorSet = Arc<ValidatorSet>;

/// Create shared validator set
pub fn create_validator_set() -> SharedValidatorSet {
    Arc::new(ValidatorSet::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    
    #[test]
    fn test_validator_set() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let kp3 = KeyPair::generate();
        
        let v1 = ValidatorInfo::new(kp1.node_id(), kp1.public_key(), 1000);
        let v2 = ValidatorInfo::new(kp2.node_id(), kp2.public_key(), 2000);
        let v3 = ValidatorInfo::new(kp3.node_id(), kp3.public_key(), 3000);
        
        let set = ValidatorSet::with_validators(vec![v1.clone(), v2, v3]);
        
        assert_eq!(set.active_count(), 3);
        assert_eq!(set.required_votes(), 3); // 2/3 of 3 + 1 = 3
        assert!(set.is_validator(&v1.node_id));
    }
    
    #[test]
    fn test_local_validator_signing() {
        let kp = KeyPair::generate();
        let local = LocalValidator::new(kp);
        
        let message = b"test message";
        let signature = local.sign(message);
        
        let set = ValidatorSet::new();
        set.add_validator(local.to_validator_info(1000));
        
        assert!(set.verify_signature(&local.node_id(), message, &signature).is_ok());
    }
}
