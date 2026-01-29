//! RELYO Ledger - Account balance management

use async_trait::async_trait;
use parking_lot::RwLock;
use rainsonet_core::{
    Address, Amount, Nonce, RainsonetError, RainsonetResult, RelyoConfig, StateChange,
};
use rainsonet_state::{AccountState, StateStore};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

use crate::transaction::{RelyoTransaction, VerifiedTransaction};

/// Account information
#[derive(Debug, Clone, Default)]
pub struct Account {
    pub address: Address,
    pub balance: Amount,
    pub nonce: Nonce,
}

impl Account {
    pub fn new(address: Address, balance: Amount) -> Self {
        Self {
            address,
            balance,
            nonce: Nonce::new(0),
        }
    }
    
    pub fn from_state(address: Address, state: AccountState) -> Self {
        Self {
            address,
            balance: Amount::new(state.balance),
            nonce: Nonce::new(state.nonce),
        }
    }
    
    pub fn to_state(&self) -> AccountState {
        AccountState::new(self.balance.0, self.nonce.0)
    }
}

/// RELYO Ledger for managing accounts
pub struct RelyoLedger<S: StateStore> {
    state: Arc<S>,
    config: RelyoConfig,
    pending_changes: RwLock<HashMap<Address, Account>>,
    total_supply: RwLock<Amount>,
    burned: RwLock<Amount>,
}

impl<S: StateStore + 'static> RelyoLedger<S> {
    pub fn new(state: Arc<S>, config: RelyoConfig) -> Self {
        Self {
            state,
            config,
            pending_changes: RwLock::new(HashMap::new()),
            total_supply: RwLock::new(Amount::ZERO),
            burned: RwLock::new(Amount::ZERO),
        }
    }
    
    /// Get account, checking pending changes first
    pub async fn get_account(&self, address: &Address) -> RainsonetResult<Account> {
        // Check pending changes first
        if let Some(account) = self.pending_changes.read().get(address) {
            return Ok(account.clone());
        }
        
        // Check state store
        match self.state.get_account(address.as_bytes()).await? {
            Some(state) => Ok(Account::from_state(*address, state)),
            None => Ok(Account::new(*address, Amount::ZERO)),
        }
    }
    
    /// Get balance
    pub async fn get_balance(&self, address: &Address) -> RainsonetResult<Amount> {
        Ok(self.get_account(address).await?.balance)
    }
    
    /// Get nonce
    pub async fn get_nonce(&self, address: &Address) -> RainsonetResult<Nonce> {
        Ok(self.get_account(address).await?.nonce)
    }
    
    /// Execute a verified transaction and return state changes
    pub async fn execute_transaction(
        &self,
        tx: &VerifiedTransaction,
    ) -> RainsonetResult<Vec<StateChange>> {
        let tx = &tx.tx;
        
        // Get current accounts
        let mut sender = self.get_account(&tx.from).await?;
        let mut recipient = self.get_account(&tx.to).await?;
        
        // Validate nonce
        if tx.nonce != sender.nonce {
            return Err(RainsonetError::InvalidNonce {
                expected: sender.nonce.0,
                got: tx.nonce.0,
            });
        }
        
        // Validate balance
        let total_cost = tx.total_cost();
        if sender.balance.0 < total_cost.0 {
            return Err(RainsonetError::InsufficientBalance {
                required: total_cost.0,
                available: sender.balance.0,
            });
        }
        
        // Calculate fee distribution
        let burn_amount = Amount::new(
            tx.fee.0 * self.config.fee_burn_percent as u128 / 100,
        );
        let validator_fee = tx.fee.saturating_sub(burn_amount);
        
        // Update sender
        sender.balance = sender.balance.saturating_sub(total_cost);
        sender.nonce = sender.nonce.next();
        
        // Update recipient
        recipient.balance = recipient.balance.saturating_add(tx.amount);
        
        // Track burned amount
        if burn_amount.0 > 0 {
            *self.burned.write() = self.burned.read().saturating_add(burn_amount);
        }
        
        // Create state changes
        let mut changes = Vec::new();
        
        changes.push(StateChange::Set {
            key: rainsonet_state::account_key(tx.from.as_bytes()),
            value: sender.to_state().to_bytes(),
        });
        
        changes.push(StateChange::Set {
            key: rainsonet_state::account_key(tx.to.as_bytes()),
            value: recipient.to_state().to_bytes(),
        });
        
        // Update pending changes
        {
            let mut pending = self.pending_changes.write();
            pending.insert(tx.from, sender);
            pending.insert(tx.to, recipient);
        }
        
        debug!(
            "Executed tx: {} -> {} amount={} fee={}",
            tx.from, tx.to, tx.amount, tx.fee
        );
        
        Ok(changes)
    }
    
    /// Commit pending changes to state
    pub async fn commit(&self) -> RainsonetResult<()> {
        let pending = std::mem::take(&mut *self.pending_changes.write());
        
        for (address, account) in pending {
            self.state
                .set_account(address.as_bytes(), &account.to_state())
                .await?;
        }
        
        Ok(())
    }
    
    /// Rollback pending changes
    pub fn rollback(&self) {
        self.pending_changes.write().clear();
    }
    
    /// Get total supply
    pub fn total_supply(&self) -> Amount {
        *self.total_supply.read()
    }
    
    /// Get total burned
    pub fn total_burned(&self) -> Amount {
        *self.burned.read()
    }
    
    /// Set initial balance (for genesis)
    pub async fn set_balance(
        &self,
        address: &Address,
        balance: Amount,
    ) -> RainsonetResult<()> {
        let account = Account::new(*address, balance);
        self.state
            .set_account(address.as_bytes(), &account.to_state())
            .await?;
        
        // Update total supply
        *self.total_supply.write() = self.total_supply.read().saturating_add(balance);
        
        info!("Set balance for {}: {}", address, balance);
        Ok(())
    }
    
    /// Get configuration
    pub fn config(&self) -> &RelyoConfig {
        &self.config
    }
}

/// Shared ledger type
pub type SharedLedger<S> = Arc<RelyoLedger<S>>;

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    use rainsonet_state::MemoryStateStore;
    
    async fn setup_ledger() -> (SharedLedger<MemoryStateStore>, KeyPair, KeyPair) {
        let state = Arc::new(MemoryStateStore::new());
        let config = RelyoConfig::default();
        let ledger = Arc::new(RelyoLedger::new(state, config));
        
        let sender = KeyPair::generate();
        let recipient = KeyPair::generate();
        
        // Give sender some balance
        ledger
            .set_balance(&sender.address(), Amount::from_relyo(1000))
            .await
            .unwrap();
        
        (ledger, sender, recipient)
    }
    
    #[tokio::test]
    async fn test_balance_management() {
        let (ledger, sender, _) = setup_ledger().await;
        
        let balance = ledger.get_balance(&sender.address()).await.unwrap();
        assert_eq!(balance.0, Amount::from_relyo(1000).0);
    }
    
    #[tokio::test]
    async fn test_transaction_execution() {
        let (ledger, sender, recipient) = setup_ledger().await;
        
        let tx = crate::transaction::RelyoTransaction::new(
            sender.address(),
            recipient.address(),
            Amount::from_relyo(100),
            Amount::new(1_000_000_000_000_000),
            Nonce::new(0),
            &sender,
        )
        .unwrap();
        
        let verified = VerifiedTransaction::new(tx).unwrap();
        let changes = ledger.execute_transaction(&verified).await.unwrap();
        
        assert!(!changes.is_empty());
        
        // Check balances (from pending changes)
        let sender_balance = ledger.get_balance(&sender.address()).await.unwrap();
        let recipient_balance = ledger.get_balance(&recipient.address()).await.unwrap();
        
        assert!(sender_balance.0 < Amount::from_relyo(1000).0);
        assert_eq!(recipient_balance.0, Amount::from_relyo(100).0);
    }
    
    #[tokio::test]
    async fn test_insufficient_balance() {
        let (ledger, sender, recipient) = setup_ledger().await;
        
        let tx = crate::transaction::RelyoTransaction::new(
            sender.address(),
            recipient.address(),
            Amount::from_relyo(2000), // More than balance
            Amount::new(1_000_000_000_000_000),
            Nonce::new(0),
            &sender,
        )
        .unwrap();
        
        let verified = VerifiedTransaction::new(tx).unwrap();
        let result = ledger.execute_transaction(&verified).await;
        
        assert!(matches!(result, Err(RainsonetError::InsufficientBalance { .. })));
    }
    
    #[tokio::test]
    async fn test_invalid_nonce() {
        let (ledger, sender, recipient) = setup_ledger().await;
        
        let tx = crate::transaction::RelyoTransaction::new(
            sender.address(),
            recipient.address(),
            Amount::from_relyo(10),
            Amount::new(1_000_000_000_000_000),
            Nonce::new(5), // Wrong nonce
            &sender,
        )
        .unwrap();
        
        let verified = VerifiedTransaction::new(tx).unwrap();
        let result = ledger.execute_transaction(&verified).await;
        
        assert!(matches!(result, Err(RainsonetError::InvalidNonce { .. })));
    }
}
