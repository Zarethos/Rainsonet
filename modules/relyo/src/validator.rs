//! Transaction validation for RELYO

use async_trait::async_trait;
use rainsonet_core::{
    Amount, RainsonetError, RainsonetResult, RelyoConfig, StateProvider, TransactionValidator,
};
use rainsonet_state::{AccountState, StateStore};
use std::sync::Arc;

use crate::transaction::RelyoTransaction;

/// RELYO Transaction Validator
pub struct RelyoTransactionValidator {
    config: RelyoConfig,
}

impl RelyoTransactionValidator {
    pub fn new(config: RelyoConfig) -> Self {
        Self { config }
    }
    
    /// Validate transaction structure
    pub fn validate_structure(&self, tx: &RelyoTransaction) -> RainsonetResult<()> {
        // Self-transfer is allowed (for nonce advancement)
        
        // Amount must not exceed max
        if tx.amount.0 > self.config.max_tx_amount {
            return Err(RainsonetError::InvalidTransaction(format!(
                "Amount {} exceeds maximum {}",
                tx.amount.0, self.config.max_tx_amount
            )));
        }
        
        // Fee must meet minimum
        if tx.fee.0 < self.config.min_fee {
            return Err(RainsonetError::FeeTooLow {
                minimum: self.config.min_fee,
                provided: tx.fee.0,
            });
        }
        
        // Check expiry
        if tx.is_expired(self.config.tx_expiry_seconds) {
            return Err(RainsonetError::TransactionExpired);
        }
        
        Ok(())
    }
    
    /// Validate transaction signature
    pub fn validate_signature(&self, tx: &RelyoTransaction) -> RainsonetResult<()> {
        tx.verify_signature()
    }
    
    /// Validate transaction against state
    pub async fn validate_against_state<S: StateStore>(
        &self,
        tx: &RelyoTransaction,
        state: &S,
    ) -> RainsonetResult<()> {
        // Get sender account
        let sender_state = state
            .get_account(tx.from.as_bytes())
            .await?
            .unwrap_or_default();
        
        // Validate nonce
        if tx.nonce.0 != sender_state.nonce {
            return Err(RainsonetError::InvalidNonce {
                expected: sender_state.nonce,
                got: tx.nonce.0,
            });
        }
        
        // Validate balance
        let total_cost = tx.total_cost();
        if sender_state.balance < total_cost.0 {
            return Err(RainsonetError::InsufficientBalance {
                required: total_cost.0,
                available: sender_state.balance,
            });
        }
        
        Ok(())
    }
    
    /// Full validation
    pub async fn validate<S: StateStore>(
        &self,
        tx: &RelyoTransaction,
        state: &S,
    ) -> RainsonetResult<()> {
        self.validate_structure(tx)?;
        self.validate_signature(tx)?;
        self.validate_against_state(tx, state).await?;
        Ok(())
    }
}

#[async_trait]
impl TransactionValidator<RelyoTransaction> for RelyoTransactionValidator {
    async fn validate(
        &self,
        tx: &RelyoTransaction,
        state: &dyn StateProvider,
    ) -> RainsonetResult<()> {
        self.validate_structure(tx)?;
        self.validate_signature(tx)?;
        
        // Get sender account from state
        let key = rainsonet_state::account_key(tx.from.as_bytes());
        let sender_state = match state.get(&key).await? {
            Some(bytes) => AccountState::from_bytes(&bytes)?,
            None => AccountState::default(),
        };
        
        // Validate nonce
        if tx.nonce.0 != sender_state.nonce {
            return Err(RainsonetError::InvalidNonce {
                expected: sender_state.nonce,
                got: tx.nonce.0,
            });
        }
        
        // Validate balance
        let total_cost = tx.total_cost();
        if sender_state.balance < total_cost.0 {
            return Err(RainsonetError::InsufficientBalance {
                required: total_cost.0,
                available: sender_state.balance,
            });
        }
        
        Ok(())
    }
}

/// Batch validator for multiple transactions
pub struct BatchValidator {
    validator: RelyoTransactionValidator,
}

impl BatchValidator {
    pub fn new(config: RelyoConfig) -> Self {
        Self {
            validator: RelyoTransactionValidator::new(config),
        }
    }
    
    /// Validate a batch of transactions
    pub async fn validate_batch<S: StateStore>(
        &self,
        transactions: &[RelyoTransaction],
        state: &S,
    ) -> Vec<RainsonetResult<()>> {
        let mut results = Vec::with_capacity(transactions.len());
        
        for tx in transactions {
            results.push(self.validator.validate(tx, state).await);
        }
        
        results
    }
    
    /// Filter valid transactions from a batch
    pub async fn filter_valid<S: StateStore>(
        &self,
        transactions: Vec<RelyoTransaction>,
        state: &S,
    ) -> Vec<RelyoTransaction> {
        let mut valid = Vec::new();
        
        for tx in transactions {
            if self.validator.validate(&tx, state).await.is_ok() {
                valid.push(tx);
            }
        }
        
        valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    use rainsonet_state::MemoryStateStore;
    
    #[tokio::test]
    async fn test_validation_success() {
        let config = RelyoConfig::default();
        let validator = RelyoTransactionValidator::new(config);
        let state = MemoryStateStore::new();
        
        let sender = KeyPair::generate();
        let recipient = KeyPair::generate();
        
        // Give sender balance
        let account = AccountState::new(Amount::from_relyo(1000).0, 0);
        state
            .set_account(sender.address().as_bytes(), &account)
            .await
            .unwrap();
        
        let tx = RelyoTransaction::new(
            sender.address(),
            recipient.address(),
            Amount::from_relyo(10),
            Amount::new(1_000_000_000_000_000),
            rainsonet_core::Nonce::new(0),
            &sender,
        )
        .unwrap();
        
        assert!(validator.validate(&tx, &state).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_fee_too_low() {
        let config = RelyoConfig::default();
        let validator = RelyoTransactionValidator::new(config.clone());
        
        let sender = KeyPair::generate();
        let recipient = KeyPair::generate();
        
        let tx = RelyoTransaction::new(
            sender.address(),
            recipient.address(),
            Amount::from_relyo(10),
            Amount::new(1), // Way too low
            rainsonet_core::Nonce::new(0),
            &sender,
        )
        .unwrap();
        
        let result = validator.validate_structure(&tx);
        assert!(matches!(result, Err(RainsonetError::FeeTooLow { .. })));
    }
}
