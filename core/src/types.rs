//! Core types for RAINSONET
//! 
//! Defines fundamental data structures used across the system.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 32-byte address derived from public key hash
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 32]);

impl Address {
    pub const ZERO: Address = Address([0u8; 32]);
    
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Address(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Address(arr))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", &self.to_hex()[..16])
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address(0x{})", self.to_hex())
    }
}

/// 32-byte hash type
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub const ZERO: Hash = Hash([0u8; 32]);
    
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", &self.to_hex()[..16])
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash(0x{})", self.to_hex())
    }
}

/// 64-byte signature
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub [u8; 64]);

impl Signature {
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Signature(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature(0x{}...)", &self.to_hex()[..16])
    }
}

/// 32-byte public key
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey(0x{})", self.to_hex())
    }
}

/// Balance amount (in smallest unit)
/// Using u128 for large amounts support
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Amount(pub u128);

impl Amount {
    pub const ZERO: Amount = Amount(0);
    pub const MAX: Amount = Amount(u128::MAX);
    
    /// One RELYO = 10^18 smallest units (like ETH wei)
    pub const DECIMALS: u32 = 18;
    pub const ONE_RELYO: u128 = 1_000_000_000_000_000_000;
    
    pub fn new(value: u128) -> Self {
        Amount(value)
    }
    
    pub fn from_relyo(relyo: u64) -> Self {
        Amount(relyo as u128 * Self::ONE_RELYO)
    }
    
    pub fn checked_add(self, other: Amount) -> Option<Amount> {
        self.0.checked_add(other.0).map(Amount)
    }
    
    pub fn checked_sub(self, other: Amount) -> Option<Amount> {
        self.0.checked_sub(other.0).map(Amount)
    }
    
    pub fn saturating_add(self, other: Amount) -> Amount {
        Amount(self.0.saturating_add(other.0))
    }
    
    pub fn saturating_sub(self, other: Amount) -> Amount {
        Amount(self.0.saturating_sub(other.0))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / Self::ONE_RELYO;
        let frac = self.0 % Self::ONE_RELYO;
        if frac == 0 {
            write!(f, "{} RELYO", whole)
        } else {
            write!(f, "{}.{:018} RELYO", whole, frac)
        }
    }
}

impl fmt::Debug for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Amount({})", self.0)
    }
}

/// Transaction nonce (sequential per account)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Nonce(pub u64);

impl Nonce {
    pub fn new(value: u64) -> Self {
        Nonce(value)
    }
    
    pub fn next(&self) -> Nonce {
        Nonce(self.0 + 1)
    }
}

impl fmt::Display for Nonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Nonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Nonce({})", self.0)
    }
}

/// Timestamp in milliseconds since Unix epoch
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self {
        Timestamp(chrono::Utc::now().timestamp_millis() as u64)
    }
    
    pub fn from_millis(millis: u64) -> Self {
        Timestamp(millis)
    }
    
    pub fn as_millis(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timestamp({})", self.0)
    }
}

/// State version for optimistic concurrency
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct StateVersion(pub u64);

impl StateVersion {
    pub fn new(value: u64) -> Self {
        StateVersion(value)
    }
    
    pub fn next(&self) -> StateVersion {
        StateVersion(self.0 + 1)
    }
}

impl fmt::Display for StateVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// Node identifier (derived from public key)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        NodeId(bytes)
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node:{}", &self.to_hex()[..12])
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.to_hex())
    }
}

/// Transaction ID (hash of transaction content)
pub type TxId = Hash;

/// State root hash
pub type StateRoot = Hash;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_address_hex() {
        let addr = Address([1u8; 32]);
        let hex = addr.to_hex();
        let parsed = Address::from_hex(&hex).unwrap();
        assert_eq!(addr, parsed);
    }
    
    #[test]
    fn test_amount_operations() {
        let a = Amount::from_relyo(10);
        let b = Amount::from_relyo(5);
        assert_eq!(a.checked_sub(b), Some(Amount::from_relyo(5)));
        assert_eq!(b.checked_sub(a), None);
    }
    
    #[test]
    fn test_nonce_sequence() {
        let n = Nonce::new(0);
        assert_eq!(n.next(), Nonce::new(1));
    }
}
