//! RAINSONET Consensus Engine
//! 
//! Implements validator-based consensus with:
//! - Fast deterministic finality
//! - Proposal-vote mechanism
//! - 2/3 majority agreement

pub mod engine;
pub mod proposal;
pub mod validator;
pub mod vote;

pub use engine::*;
pub use proposal::*;
pub use validator::*;
pub use vote::*;
