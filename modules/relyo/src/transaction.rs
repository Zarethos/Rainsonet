//! RELYO Transaction implementation

use rainsonet_core::{
    Address, Amount, Hash, Hashable, Nonce, PublicKey, RainsonetError, RainsonetResult,
    Signable, Signature, Timestamp, Transaction as TransactionTrait,
};
use rainsonet_crypto::hashing::hash;
use rainsonet_crypto::keys::{address_from_public_key, verify_address};
use rainsonet_crypto::signing::{sign, verify};
use serde::{Deserialize, Serialize};

/// RELYO Transaction
/// 
/// Format:
/// - from_address: sender
/// - to_address: recipient  
/// - amount: transfer amount
/// - fee: transaction fee
/// - nonce: sequential per account
/// - signature: Ed25519 signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelyoTransaction {
    /// Sender address
    pub from: Address,
    /// Recipient address
    pub to: Address,
    /// Transfer amount (smallest unit)
    pub amount: Amount,
    /// Transaction fee (smallest unit)
    pub fee: Amount,
    /// Sequential nonce for sender
    pub nonce: Nonce,
    /// Transaction timestamp
    pub timestamp: Timestamp,
    /// Sender's public key
    pub public_key: PublicKey,
    /// Transaction signature
    pub signature: Signature,
}

impl RelyoTransaction {
    /// Create and sign a new transaction
    pub fn new(
        from: Address,
        to: Address,
        amount: Amount,
        fee: Amount,
        nonce: Nonce,
        keypair: &rainsonet_crypto::keys::KeyPair,
    ) -> RainsonetResult<Self> {
        // Verify address matches keypair
        let derived_address = address_from_public_key(&keypair.public_key());
        if from != derived_address {
            return Err(RainsonetError::InvalidAddress(
                "Address does not match public key".into(),
            ));
        }
        
        let timestamp = Timestamp::now();
        let public_key = keypair.public_key();
        
        // Create unsigned transaction for signing
        let signing_bytes = Self::compute_signing_bytes(
            &from, &to, amount, fee, nonce, timestamp,
        );
        
        let signature = sign(keypair, &signing_bytes);
        
        Ok(Self {
            from,
            to,
            amount,
            fee,
            nonce,
            timestamp,
            public_key,
            signature,
        })
    }
    
    /// Compute bytes to sign
    fn compute_signing_bytes(
        from: &Address,
        to: &Address,
        amount: Amount,
        fee: Amount,
        nonce: Nonce,
        timestamp: Timestamp,
    ) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(200);
        bytes.extend_from_slice(b"RELYO_TX:");
        bytes.extend_from_slice(from.as_bytes());
        bytes.extend_from_slice(to.as_bytes());
        bytes.extend_from_slice(&amount.0.to_le_bytes());
        bytes.extend_from_slice(&fee.0.to_le_bytes());
        bytes.extend_from_slice(&nonce.0.to_le_bytes());
        bytes.extend_from_slice(&timestamp.0.to_le_bytes());
        bytes
    }
    
    /// Verify the transaction signature
    pub fn verify_signature(&self) -> RainsonetResult<()> {
        // Verify address matches public key
        if !verify_address(&self.from, &self.public_key) {
            return Err(RainsonetError::InvalidAddress(
                "Address does not match public key".into(),
            ));
        }
        
        // Verify signature
        let signing_bytes = Self::compute_signing_bytes(
            &self.from,
            &self.to,
            self.amount,
            self.fee,
            self.nonce,
            self.timestamp,
        );
        
        verify(&self.public_key, &signing_bytes, &self.signature)
    }
    
    /// Total amount deducted from sender (amount + fee)
    pub fn total_cost(&self) -> Amount {
        self.amount.saturating_add(self.fee)
    }
    
    /// Check if transaction is expired
    pub fn is_expired(&self, expiry_seconds: u64) -> bool {
        let now = Timestamp::now();
        let expiry_ms = expiry_seconds * 1000;
        now.as_millis() - self.timestamp.as_millis() > expiry_ms
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> RainsonetResult<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| RainsonetError::DeserializationError(e.to_string()))
    }
}

impl Hashable for RelyoTransaction {
    fn hash(&self) -> Hash {
        hash(&self.to_bytes())
    }
}

impl Signable for RelyoTransaction {
    fn signing_bytes(&self) -> Vec<u8> {
        Self::compute_signing_bytes(
            &self.from,
            &self.to,
            self.amount,
            self.fee,
            self.nonce,
            self.timestamp,
        )
    }
}

impl TransactionTrait for RelyoTransaction {
    fn sender(&self) -> Address {
        self.from
    }
    
    fn nonce(&self) -> Nonce {
        self.nonce
    }
    
    fn fee(&self) -> Amount {
        self.fee
    }
    
    fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

/// Transaction builder for easier construction
pub struct TransactionBuilder {
    from: Option<Address>,
    to: Option<Address>,
    amount: Amount,
    fee: Amount,
    nonce: Option<Nonce>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            from: None,
            to: None,
            amount: Amount::ZERO,
            fee: Amount::new(1_000_000_000_000_000), // Default 0.001 RELYO
            nonce: None,
        }
    }
    
    pub fn from(mut self, address: Address) -> Self {
        self.from = Some(address);
        self
    }
    
    pub fn to(mut self, address: Address) -> Self {
        self.to = Some(address);
        self
    }
    
    pub fn amount(mut self, amount: Amount) -> Self {
        self.amount = amount;
        self
    }
    
    pub fn amount_relyo(mut self, relyo: u64) -> Self {
        self.amount = Amount::from_relyo(relyo);
        self
    }
    
    pub fn fee(mut self, fee: Amount) -> Self {
        self.fee = fee;
        self
    }
    
    pub fn nonce(mut self, nonce: Nonce) -> Self {
        self.nonce = Some(nonce);
        self
    }
    
    pub fn build(
        self,
        keypair: &rainsonet_crypto::keys::KeyPair,
    ) -> RainsonetResult<RelyoTransaction> {
        let from = self.from.ok_or(RainsonetError::InvalidTransaction(
            "Missing from address".into(),
        ))?;
        let to = self.to.ok_or(RainsonetError::InvalidTransaction(
            "Missing to address".into(),
        ))?;
        let nonce = self.nonce.ok_or(RainsonetError::InvalidTransaction(
            "Missing nonce".into(),
        ))?;
        
        RelyoTransaction::new(from, to, self.amount, self.fee, nonce, keypair)
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Signed transaction with verification status
#[derive(Debug, Clone)]
pub struct VerifiedTransaction {
    pub tx: RelyoTransaction,
    pub tx_id: Hash,
}

impl VerifiedTransaction {
    /// Create and verify a transaction
    pub fn new(tx: RelyoTransaction) -> RainsonetResult<Self> {
        tx.verify_signature()?;
        let tx_id = tx.hash();
        Ok(Self { tx, tx_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    
    #[test]
    fn test_transaction_creation() {
        let sender_kp = KeyPair::generate();
        let recipient_kp = KeyPair::generate();
        
        let tx = RelyoTransaction::new(
            sender_kp.address(),
            recipient_kp.address(),
            Amount::from_relyo(10),
            Amount::new(1_000_000_000_000_000),
            Nonce::new(0),
            &sender_kp,
        )
        .unwrap();
        
        assert!(tx.verify_signature().is_ok());
    }
    
    #[test]
    fn test_transaction_builder() {
        let sender_kp = KeyPair::generate();
        let recipient_kp = KeyPair::generate();
        
        let tx = TransactionBuilder::new()
            .from(sender_kp.address())
            .to(recipient_kp.address())
            .amount_relyo(100)
            .nonce(Nonce::new(0))
            .build(&sender_kp)
            .unwrap();
        
        assert!(tx.verify_signature().is_ok());
        assert_eq!(tx.amount.0, Amount::from_relyo(100).0);
    }
    
    #[test]
    fn test_invalid_signature() {
        let sender_kp = KeyPair::generate();
        let other_kp = KeyPair::generate();
        
        // Create transaction with wrong keypair
        let mut tx = RelyoTransaction::new(
            sender_kp.address(),
            other_kp.address(),
            Amount::from_relyo(10),
            Amount::new(1_000_000_000_000_000),
            Nonce::new(0),
            &sender_kp,
        )
        .unwrap();
        
        // Tamper with amount
        tx.amount = Amount::from_relyo(1000);
        
        assert!(tx.verify_signature().is_err());
    }
    
    #[test]
    fn test_total_cost() {
        let tx_amount = Amount::from_relyo(10);
        let tx_fee = Amount::new(1_000_000_000_000_000);
        
        let kp = KeyPair::generate();
        let tx = RelyoTransaction::new(
            kp.address(),
            kp.address(),
            tx_amount,
            tx_fee,
            Nonce::new(0),
            &kp,
        )
        .unwrap();
        
        assert_eq!(tx.total_cost(), tx_amount.saturating_add(tx_fee));
    }
}
