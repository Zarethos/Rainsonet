//! Key management for RAINSONET
//! 
//! Handles key generation, storage, and address derivation.

use ed25519_dalek::{
    SigningKey as Ed25519SigningKey,
    VerifyingKey as Ed25519VerifyingKey,
    SECRET_KEY_LENGTH,
};
use rainsonet_core::{Address, NodeId, PublicKey, RainsonetError, RainsonetResult};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::hashing::blake3_hash;

/// A keypair for signing and verification
#[derive(Clone)]
pub struct KeyPair {
    signing_key: Ed25519SigningKey,
}

impl KeyPair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let signing_key = Ed25519SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }
    
    /// Create keypair from seed bytes
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = Ed25519SigningKey::from_bytes(seed);
        Self { signing_key }
    }
    
    /// Create keypair from secret key bytes
    pub fn from_secret_bytes(bytes: &[u8]) -> RainsonetResult<Self> {
        if bytes.len() != SECRET_KEY_LENGTH {
            return Err(RainsonetError::InvalidPrivateKey);
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(bytes);
        Ok(Self::from_seed(&seed))
    }
    
    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        let verifying_key = self.signing_key.verifying_key();
        PublicKey::from_bytes(verifying_key.to_bytes())
    }
    
    /// Get the address (hash of public key)
    pub fn address(&self) -> Address {
        address_from_public_key(&self.public_key())
    }
    
    /// Get the node ID (same as address for nodes)
    pub fn node_id(&self) -> NodeId {
        let addr = self.address();
        NodeId::from_bytes(*addr.as_bytes())
    }
    
    /// Get the secret key bytes (BE CAREFUL with this!)
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
    
    /// Get the internal signing key for signing operations
    pub(crate) fn signing_key(&self) -> &Ed25519SigningKey {
        &self.signing_key
    }
}

impl Drop for KeyPair {
    fn drop(&mut self) {
        // Zeroize is handled by ed25519-dalek internally
    }
}

/// Derive address from public key using BLAKE3 hash
pub fn address_from_public_key(public_key: &PublicKey) -> Address {
    let hash = blake3_hash(public_key.as_bytes());
    Address::from_bytes(*hash.as_bytes())
}

/// Verify that an address matches a public key
pub fn verify_address(address: &Address, public_key: &PublicKey) -> bool {
    let derived = address_from_public_key(public_key);
    address == &derived
}

/// Convert Ed25519 verifying key to our PublicKey type
pub fn public_key_from_ed25519(key: &Ed25519VerifyingKey) -> PublicKey {
    PublicKey::from_bytes(key.to_bytes())
}

/// Convert our PublicKey type to Ed25519 verifying key
pub fn public_key_to_ed25519(key: &PublicKey) -> RainsonetResult<Ed25519VerifyingKey> {
    Ed25519VerifyingKey::from_bytes(key.as_bytes())
        .map_err(|_| RainsonetError::InvalidPublicKey)
}

/// Secure secret key storage (zeroizes on drop)
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    bytes: [u8; 32],
}

impl SecretKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
    
    pub fn to_keypair(&self) -> KeyPair {
        KeyPair::from_seed(&self.bytes)
    }
}

/// Serializable public key info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyInfo {
    pub public_key: String,
    pub address: String,
}

impl From<&KeyPair> for PublicKeyInfo {
    fn from(keypair: &KeyPair) -> Self {
        Self {
            public_key: keypair.public_key().to_hex(),
            address: keypair.address().to_hex(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        let pubkey = kp.public_key();
        let addr = kp.address();
        
        // Verify address derivation
        assert!(verify_address(&addr, &pubkey));
    }
    
    #[test]
    fn test_keypair_from_seed() {
        let seed = [42u8; 32];
        let kp1 = KeyPair::from_seed(&seed);
        let kp2 = KeyPair::from_seed(&seed);
        
        assert_eq!(kp1.public_key(), kp2.public_key());
        assert_eq!(kp1.address(), kp2.address());
    }
    
    #[test]
    fn test_secret_key_zeroize() {
        let secret = SecretKey::new([42u8; 32]);
        let kp = secret.to_keypair();
        assert!(kp.public_key().as_bytes() != &[0u8; 32]);
    }
}
