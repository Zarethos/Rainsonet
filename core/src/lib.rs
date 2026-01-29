//! RAINSONET Core Library
//! 
//! Core types, traits, and abstractions for the RAINSONET decentralized payment rail.
//! This crate provides the foundation for all other RAINSONET components.

pub mod types;
pub mod traits;
pub mod error;
pub mod config;

pub use types::*;
pub use traits::*;
pub use error::*;
pub use config::*;
