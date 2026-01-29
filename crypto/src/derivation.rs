//! Key derivation using HKDF

use hkdf::Hkdf;
use rainsonet_core::{RainsonetError, RainsonetResult};
use sha2::Sha256;

use crate::keys::{KeyPair, SecretKey};

/// Derive a key using HKDF-SHA256
pub fn derive_key(
    input_key_material: &[u8],
    salt: Option<&[u8]>,
    info: &[u8],
    output_length: usize,
) -> RainsonetResult<Vec<u8>> {
    let hk = Hkdf::<Sha256>::new(salt, input_key_material);
    let mut output = vec![0u8; output_length];
    
    hk.expand(info, &mut output)
        .map_err(|e| RainsonetError::KeyDerivationFailed(e.to_string()))?;
    
    Ok(output)
}

/// Derive a 32-byte key
pub fn derive_key_32(
    input_key_material: &[u8],
    salt: Option<&[u8]>,
    info: &[u8],
) -> RainsonetResult<[u8; 32]> {
    let key = derive_key(input_key_material, salt, info, 32)?;
    let mut result = [0u8; 32];
    result.copy_from_slice(&key);
    Ok(result)
}

/// Derive a keypair from a seed and path
pub fn derive_keypair(
    seed: &[u8],
    path: &str,
) -> RainsonetResult<KeyPair> {
    let derived = derive_key_32(seed, None, path.as_bytes())?;
    Ok(KeyPair::from_seed(&derived))
}

/// Derive a secret key from a seed and index
pub fn derive_secret_key(
    seed: &[u8],
    index: u32,
) -> RainsonetResult<SecretKey> {
    let info = format!("rainsonet/key/{}", index);
    let derived = derive_key_32(seed, None, info.as_bytes())?;
    Ok(SecretKey::new(derived))
}

/// Master key for hierarchical derivation
pub struct MasterKey {
    seed: [u8; 32],
}

impl MasterKey {
    /// Create from a 32-byte seed
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self { seed }
    }
    
    /// Generate a random master key
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut seed = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut seed);
        Self { seed }
    }
    
    /// Create from a mnemonic phrase (simple implementation)
    pub fn from_phrase(phrase: &str) -> RainsonetResult<Self> {
        let seed = derive_key_32(phrase.as_bytes(), Some(b"rainsonet-seed"), b"master")?;
        Ok(Self { seed })
    }
    
    /// Derive a keypair at the given index
    pub fn derive_keypair(&self, index: u32) -> RainsonetResult<KeyPair> {
        let secret = derive_secret_key(&self.seed, index)?;
        Ok(secret.to_keypair())
    }
    
    /// Derive a keypair at a custom path
    pub fn derive_path(&self, path: &str) -> RainsonetResult<KeyPair> {
        derive_keypair(&self.seed, path)
    }
    
    /// Get the seed bytes (BE CAREFUL!)
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        // Zeroize the seed
        self.seed.iter_mut().for_each(|b| *b = 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_derive_key() {
        let ikm = b"input key material";
        let salt = b"salt";
        let info = b"context info";
        
        let key1 = derive_key(ikm, Some(salt), info, 32).unwrap();
        let key2 = derive_key(ikm, Some(salt), info, 32).unwrap();
        
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
    }
    
    #[test]
    fn test_derive_keypair() {
        let seed = b"my secret seed for derivation!!!!";
        
        let kp1 = derive_keypair(seed, "m/0").unwrap();
        let kp2 = derive_keypair(seed, "m/0").unwrap();
        let kp3 = derive_keypair(seed, "m/1").unwrap();
        
        assert_eq!(kp1.public_key(), kp2.public_key());
        assert_ne!(kp1.public_key(), kp3.public_key());
    }
    
    #[test]
    fn test_master_key() {
        let master = MasterKey::from_phrase("my secret phrase").unwrap();
        
        let kp0 = master.derive_keypair(0).unwrap();
        let kp1 = master.derive_keypair(1).unwrap();
        
        assert_ne!(kp0.public_key(), kp1.public_key());
        
        // Deterministic
        let master2 = MasterKey::from_phrase("my secret phrase").unwrap();
        let kp0_again = master2.derive_keypair(0).unwrap();
        
        assert_eq!(kp0.public_key(), kp0_again.public_key());
    }
}
