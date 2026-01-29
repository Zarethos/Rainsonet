//! RAINSONET State Management
//! 
//! Provides state storage, versioning, and state root computation.
//! Uses a key-value model where state = { key → value }
//!
//! # Stores
//! - `MemoryStateStore`: In-memory store for testing and light nodes
//! - `PersistentStateStore`: Sled-backed persistent storage
//!
//! # Snapshots
//! - `StateSnapshot`: Point-in-time state snapshots for sync

pub mod memory;
pub mod persistent;
pub mod snapshot;
pub mod store;

pub use memory::{create_memory_store, MemoryStateStore, SharedMemoryStateStore};
pub use persistent::{create_persistent_store, PersistentStateStore, SharedPersistentStateStore};
pub use snapshot::{SnapshotManager, StateSnapshot};
pub use store::{
    account_key, compute_state_root, parse_account_key, AccountState, StateBatch,
    StateChangeOp, StateDiff, StateEntry,
};
