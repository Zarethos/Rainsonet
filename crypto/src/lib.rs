//! RAINSONET Cryptography Module
//! 
//! Provides cryptographic primitives using standard, audited algorithms:
//! - Ed25519 for signatures
//! - BLAKE3 for hashing (SHA-256 fallback)
//! - HKDF for key derivation
//! - Noise Protocol for network encryption

pub mod keys;
pub mod signing;
pub mod hashing;
pub mod derivation;

pub use keys::*;
pub use signing::*;
pub use hashing::*;
pub use derivation::*;
