//! RAINSONET State Management
//! 
//! Provides state storage, versioning, and state root computation.
//! Uses a key-value model where state = { key → value }

pub mod store;
pub mod memory;
pub mod persistent;
pub mod snapshot;

pub use store::*;
pub use memory::*;
pub use persistent::*;
pub use snapshot::*;
