//! Response data models for Minecraft server queries.
//!
//! Each protocol variant produces its own response type with the relevant
//! data fields. All response types derive `Debug`, `Clone`, and `Serialize`.

pub mod java;
pub mod bedrock;
pub mod legacy;
pub mod query;
pub mod forge;
pub mod raw;
