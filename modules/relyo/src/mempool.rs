//! Transaction mempool for pending transactions

use parking_lot::RwLock;
use rainsonet_core::{Address, Amount, Hash, Hashable, Nonce, RainsonetResult, Timestamp};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::transaction::{RelyoTransaction, VerifiedTransaction};

/// Mempool entry with metadata
#[derive(Debug, Clone)]
pub struct MempoolEntry {
    pub tx: VerifiedTransaction,
    pub received_at: Timestamp,
    pub priority: u64,
}

impl MempoolEntry {
    pub fn new(tx: VerifiedTransaction) -> Self {
        // Priority based on fee (higher fee = higher priority)
        let priority = tx.tx.fee.0 as u64;
        
        Self {
            tx,
            received_at: Timestamp::now(),
            priority,
        }
    }
}

/// Transaction mempool
/// 
/// Manages pending transactions before inclusion in state updates
pub struct Mempool {
    /// Transactions by ID
    transactions: RwLock<HashMap<Hash, MempoolEntry>>,
    /// Transactions by sender address
    by_sender: RwLock<HashMap<Address, HashSet<Hash>>>,
    /// Transactions ordered by priority
    by_priority: RwLock<BTreeMap<(u64, Hash), Hash>>,
    /// Maximum pool size
    max_size: usize,
    /// Maximum transactions per sender
    max_per_sender: usize,
}

impl Mempool {
    pub fn new(max_size: usize, max_per_sender: usize) -> Self {
        Self {
            transactions: RwLock::new(HashMap::new()),
            by_sender: RwLock::new(HashMap::new()),
            by_priority: RwLock::new(BTreeMap::new()),
            max_size,
            max_per_sender,
        }
    }
    
    /// Add a transaction to the mempool
    pub fn add(&self, tx: VerifiedTransaction) -> RainsonetResult<bool> {
        let tx_id = tx.tx_id;
        let sender = tx.tx.from;
        
        let mut transactions = self.transactions.write();
        
        // Check if already exists
        if transactions.contains_key(&tx_id) {
            return Ok(false);
        }
        
        // Check pool size
        if transactions.len() >= self.max_size {
            // Try to evict lowest priority
            if !self.evict_lowest_priority() {
                warn!("Mempool full, transaction rejected");
                return Ok(false);
            }
        }
        
        // Check per-sender limit
        {
            let by_sender = self.by_sender.read();
            if let Some(sender_txs) = by_sender.get(&sender) {
                if sender_txs.len() >= self.max_per_sender {
                    warn!("Too many transactions from sender {}", sender);
                    return Ok(false);
                }
            }
        }
        
        let entry = MempoolEntry::new(tx);
        let priority = entry.priority;
        
        // Add to all indexes
        transactions.insert(tx_id, entry);
        
        self.by_sender
            .write()
            .entry(sender)
            .or_insert_with(HashSet::new)
            .insert(tx_id);
        
        self.by_priority
            .write()
            .insert((priority, tx_id), tx_id);
        
        debug!("Added transaction {} to mempool (priority: {})", tx_id, priority);
        
        Ok(true)
    }
    
    /// Remove a transaction
    pub fn remove(&self, tx_id: &Hash) -> Option<MempoolEntry> {
        let mut transactions = self.transactions.write();
        
        if let Some(entry) = transactions.remove(tx_id) {
            let sender = entry.tx.tx.from;
            
            // Remove from sender index
            let mut by_sender = self.by_sender.write();
            if let Some(sender_txs) = by_sender.get_mut(&sender) {
                sender_txs.remove(tx_id);
                if sender_txs.is_empty() {
                    by_sender.remove(&sender);
                }
            }
            
            // Remove from priority index
            self.by_priority
                .write()
                .remove(&(entry.priority, *tx_id));
            
            debug!("Removed transaction {} from mempool", tx_id);
            
            return Some(entry);
        }
        
        None
    }
    
    /// Get a transaction
    pub fn get(&self, tx_id: &Hash) -> Option<VerifiedTransaction> {
        self.transactions
            .read()
            .get(tx_id)
            .map(|e| e.tx.clone())
    }
    
    /// Check if transaction exists
    pub fn contains(&self, tx_id: &Hash) -> bool {
        self.transactions.read().contains_key(tx_id)
    }
    
    /// Get transactions for a sender
    pub fn get_by_sender(&self, sender: &Address) -> Vec<VerifiedTransaction> {
        let by_sender = self.by_sender.read();
        let transactions = self.transactions.read();
        
        by_sender
            .get(sender)
            .map(|tx_ids| {
                tx_ids
                    .iter()
                    .filter_map(|id| transactions.get(id))
                    .map(|e| e.tx.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get next nonce for sender (current nonce + pending tx count)
    pub fn get_pending_nonce(&self, sender: &Address, current_nonce: Nonce) -> Nonce {
        let by_sender = self.by_sender.read();
        let pending_count = by_sender
            .get(sender)
            .map(|txs| txs.len() as u64)
            .unwrap_or(0);
        
        Nonce::new(current_nonce.0 + pending_count)
    }
    
    /// Get highest priority transactions for block
    pub fn get_highest_priority(&self, limit: usize) -> Vec<VerifiedTransaction> {
        let by_priority = self.by_priority.read();
        let transactions = self.transactions.read();
        
        by_priority
            .iter()
            .rev()
            .take(limit)
            .filter_map(|(_, tx_id)| transactions.get(tx_id))
            .map(|e| e.tx.clone())
            .collect()
    }
    
    /// Get transactions ordered for execution (by sender nonce)
    pub fn get_executable(&self, limit: usize) -> Vec<VerifiedTransaction> {
        let transactions = self.transactions.read();
        let by_sender = self.by_sender.read();
        
        let mut result = Vec::new();
        let mut collected_by_sender: HashMap<Address, Vec<&MempoolEntry>> = HashMap::new();
        
        // Group by sender
        for entry in transactions.values() {
            collected_by_sender
                .entry(entry.tx.tx.from)
                .or_insert_with(Vec::new)
                .push(entry);
        }
        
        // Sort each sender's transactions by nonce
        for txs in collected_by_sender.values_mut() {
            txs.sort_by_key(|e| e.tx.tx.nonce.0);
        }
        
        // Interleave transactions fairly, respecting nonce order
        let mut round_robin: Vec<_> = collected_by_sender.values_mut().collect();
        let mut i = 0;
        
        while result.len() < limit && !round_robin.is_empty() {
            if let Some(entry) = round_robin[i].first() {
                result.push(entry.tx.clone());
                round_robin[i].remove(0);
            }
            
            if round_robin[i].is_empty() {
                round_robin.remove(i);
                if !round_robin.is_empty() {
                    i %= round_robin.len();
                }
            } else {
                i = (i + 1) % round_robin.len();
            }
        }
        
        result
    }
    
    /// Evict lowest priority transaction
    fn evict_lowest_priority(&self) -> bool {
        let mut by_priority = self.by_priority.write();
        
        if let Some(((_, tx_id), _)) = by_priority.iter().next().map(|(k, v)| (*k, *v)) {
            drop(by_priority);
            self.remove(&tx_id);
            return true;
        }
        
        false
    }
    
    /// Remove expired transactions
    pub fn remove_expired(&self, expiry_seconds: u64) -> Vec<Hash> {
        let now = Timestamp::now();
        let expiry_ms = expiry_seconds * 1000;
        
        let expired: Vec<Hash> = self
            .transactions
            .read()
            .iter()
            .filter(|(_, entry)| {
                now.as_millis() - entry.received_at.as_millis() > expiry_ms
            })
            .map(|(id, _)| *id)
            .collect();
        
        for id in &expired {
            self.remove(id);
        }
        
        expired
    }
    
    /// Clear all transactions
    pub fn clear(&self) {
        self.transactions.write().clear();
        self.by_sender.write().clear();
        self.by_priority.write().clear();
    }
    
    /// Get pool size
    pub fn size(&self) -> usize {
        self.transactions.read().len()
    }
    
    /// Get all transaction IDs
    pub fn all_tx_ids(&self) -> Vec<Hash> {
        self.transactions.read().keys().copied().collect()
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new(10000, 100)
    }
}

/// Shared mempool
pub type SharedMempool = Arc<Mempool>;

/// Create shared mempool
pub fn create_mempool(max_size: usize, max_per_sender: usize) -> SharedMempool {
    Arc::new(Mempool::new(max_size, max_per_sender))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rainsonet_crypto::keys::KeyPair;
    
    fn create_test_tx(sender: &KeyPair, recipient: &KeyPair, nonce: u64, fee: u128) -> VerifiedTransaction {
        let tx = RelyoTransaction::new(
            sender.address(),
            recipient.address(),
            Amount::from_relyo(10),
            Amount::new(fee),
            Nonce::new(nonce),
            sender,
        )
        .unwrap();
        
        VerifiedTransaction::new(tx).unwrap()
    }
    
    #[test]
    fn test_mempool_add_remove() {
        let mempool = Mempool::new(100, 10);
        let sender = KeyPair::generate();
        let recipient = KeyPair::generate();
        
        let tx = create_test_tx(&sender, &recipient, 0, 1_000_000_000_000_000);
        let tx_id = tx.tx_id;
        
        assert!(mempool.add(tx).unwrap());
        assert!(mempool.contains(&tx_id));
        
        mempool.remove(&tx_id);
        assert!(!mempool.contains(&tx_id));
    }
    
    #[test]
    fn test_mempool_priority() {
        let mempool = Mempool::new(100, 10);
        let sender = KeyPair::generate();
        let recipient = KeyPair::generate();
        
        // Add transactions with different fees
        let tx_low = create_test_tx(&sender, &recipient, 0, 1_000_000_000_000_000);
        let tx_high = create_test_tx(&sender, &recipient, 1, 10_000_000_000_000_000);
        
        mempool.add(tx_low).unwrap();
        mempool.add(tx_high.clone()).unwrap();
        
        let highest = mempool.get_highest_priority(1);
        assert_eq!(highest.len(), 1);
        assert_eq!(highest[0].tx_id, tx_high.tx_id);
    }
    
    #[test]
    fn test_mempool_per_sender_limit() {
        let mempool = Mempool::new(100, 2);
        let sender = KeyPair::generate();
        let recipient = KeyPair::generate();
        
        assert!(mempool.add(create_test_tx(&sender, &recipient, 0, 1_000_000_000_000_000)).unwrap());
        assert!(mempool.add(create_test_tx(&sender, &recipient, 1, 1_000_000_000_000_000)).unwrap());
        assert!(!mempool.add(create_test_tx(&sender, &recipient, 2, 1_000_000_000_000_000)).unwrap());
    }
}
