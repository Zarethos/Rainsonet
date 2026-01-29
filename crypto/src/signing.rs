//! Digital signature operations using Ed25519

use ed25519_dalek::{Signer, Verifier};
use rainsonet_core::{PublicKey, RainsonetError, RainsonetResult, Signature};

use crate::keys::{public_key_to_ed25519, KeyPair};

/// Sign a message using Ed25519
pub fn sign(keypair: &KeyPair, message: &[u8]) -> Signature {
    let signature = keypair.signing_key().sign(message);
    Signature::from_bytes(signature.to_bytes())
}

/// Verify a signature using Ed25519
pub fn verify(public_key: &PublicKey, message: &[u8], signature: &Signature) -> RainsonetResult<()> {
    let verifying_key = public_key_to_ed25519(public_key)?;
    let sig = ed25519_dalek::Signature::from_bytes(signature.as_bytes());
    
    verifying_key
        .verify(message, &sig)
        .map_err(|_| RainsonetError::InvalidSignature)
}

/// Check if a signature is valid (returns bool instead of Result)
pub fn is_valid_signature(public_key: &PublicKey, message: &[u8], signature: &Signature) -> bool {
    verify(public_key, message, signature).is_ok()
}

/// Signed message wrapper
#[derive(Debug, Clone)]
pub struct SignedMessage {
    pub message: Vec<u8>,
    pub public_key: PublicKey,
    pub signature: Signature,
}

impl SignedMessage {
    /// Create a new signed message
    pub fn new(keypair: &KeyPair, message: Vec<u8>) -> Self {
        let signature = sign(keypair, &message);
        Self {
            message,
            public_key: keypair.public_key(),
            signature,
        }
    }
    
    /// Verify this signed message
    pub fn verify(&self) -> RainsonetResult<()> {
        verify(&self.public_key, &self.message, &self.signature)
    }
    
    /// Check if this message is valid
    pub fn is_valid(&self) -> bool {
        self.verify().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"Hello, RAINSONET!";
        
        let signature = sign(&keypair, message);
        assert!(verify(&keypair.public_key(), message, &signature).is_ok());
    }
    
    #[test]
    fn test_invalid_signature() {
        let keypair1 = KeyPair::generate();
        let keypair2 = KeyPair::generate();
        let message = b"Hello, RAINSONET!";
        
        let signature = sign(&keypair1, message);
        
        // Wrong public key should fail
        assert!(verify(&keypair2.public_key(), message, &signature).is_err());
        
        // Wrong message should fail
        assert!(verify(&keypair1.public_key(), b"Different message", &signature).is_err());
    }
    
    #[test]
    fn test_signed_message() {
        let keypair = KeyPair::generate();
        let message = b"Transaction data".to_vec();
        
        let signed = SignedMessage::new(&keypair, message);
        assert!(signed.is_valid());
    }
}
