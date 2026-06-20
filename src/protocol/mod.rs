//! Protocol client implementations for each Minecraft server variant.
//!
//! Each module handles the low-level packet construction, sending, and
//! response parsing for a specific protocol version.

pub mod java;
pub mod bedrock;
pub mod legacy;
pub mod query;
