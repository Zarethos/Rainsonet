//! Hashing functions using BLAKE3 (with SHA-256 fallback)

use rainsonet_core::Hash;
use sha2::{Digest, Sha256};

/// Compute BLAKE3 hash of data
pub fn blake3_hash(data: &[u8]) -> Hash {
    let hash = blake3::hash(data);
    Hash::from_bytes(*hash.as_bytes())
}

/// Compute SHA-256 hash of data (fallback)
pub fn sha256_hash(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Hash::from_bytes(bytes)
}

/// Default hash function (BLAKE3)
pub fn hash(data: &[u8]) -> Hash {
    blake3_hash(data)
}

/// Hash multiple pieces of data
pub fn hash_multiple(parts: &[&[u8]]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    let hash = hasher.finalize();
    Hash::from_bytes(*hash.as_bytes())
}

/// Merkle tree root computation
pub fn merkle_root(leaves: &[Hash]) -> Hash {
    if leaves.is_empty() {
        return Hash::ZERO;
    }
    
    if leaves.len() == 1 {
        return leaves[0];
    }
    
    let mut current_level: Vec<Hash> = leaves.to_vec();
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        for chunk in current_level.chunks(2) {
            let hash = if chunk.len() == 2 {
                hash_multiple(&[chunk[0].as_bytes(), chunk[1].as_bytes()])
            } else {
                // Odd number: hash with itself
                hash_multiple(&[chunk[0].as_bytes(), chunk[0].as_bytes()])
            };
            next_level.push(hash);
        }
        
        current_level = next_level;
    }
    
    current_level[0]
}

/// Incremental hasher for large data
pub struct IncrementalHasher {
    hasher: blake3::Hasher,
}

impl IncrementalHasher {
    pub fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new(),
        }
    }
    
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }
    
    pub fn finalize(self) -> Hash {
        let hash = self.hasher.finalize();
        Hash::from_bytes(*hash.as_bytes())
    }
}

impl Default for IncrementalHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_blake3_hash() {
        let data = b"Hello, RAINSONET!";
        let hash1 = blake3_hash(data);
        let hash2 = blake3_hash(data);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, Hash::ZERO);
    }
    
    #[test]
    fn test_sha256_hash() {
        let data = b"Hello, RAINSONET!";
        let hash1 = sha256_hash(data);
        let hash2 = sha256_hash(data);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, Hash::ZERO);
    }
    
    #[test]
    fn test_different_hashes() {
        let data = b"Hello, RAINSONET!";
        let blake3 = blake3_hash(data);
        let sha256 = sha256_hash(data);
        
        // Different algorithms should produce different hashes
        assert_ne!(blake3, sha256);
    }
    
    #[test]
    fn test_merkle_root() {
        let leaves = vec![
            hash(b"leaf1"),
            hash(b"leaf2"),
            hash(b"leaf3"),
            hash(b"leaf4"),
        ];
        
        let root = merkle_root(&leaves);
        assert_ne!(root, Hash::ZERO);
        
        // Root should be deterministic
        let root2 = merkle_root(&leaves);
        assert_eq!(root, root2);
    }
    
    #[test]
    fn test_merkle_root_empty() {
        let root = merkle_root(&[]);
        assert_eq!(root, Hash::ZERO);
    }
    
    #[test]
    fn test_incremental_hasher() {
        let mut hasher = IncrementalHasher::new();
        hasher.update(b"Hello, ");
        hasher.update(b"RAINSONET!");
        let hash1 = hasher.finalize();
        
        let hash2 = hash(b"Hello, RAINSONET!");
        
        assert_eq!(hash1, hash2);
    }
}
