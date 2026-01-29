//! RAINSONET P2P Networking
//! 
//! Provides peer-to-peer networking using libp2p with:
//! - Noise protocol for encryption
//! - Gossipsub for message propagation
//! - mDNS for local peer discovery

pub mod network;
pub mod behaviour;
pub mod message;
pub mod peer;

pub use network::*;
pub use behaviour::*;
pub use message::*;
pub use peer::*;
