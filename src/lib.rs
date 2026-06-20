//! # mcstatus
//!
//! A library to query Minecraft servers for their status and capabilities.
//!
//! Supports:
//! - Java Edition (1.7+) via the Server List Ping protocol
//! - Legacy Java Edition (pre-1.7) via the legacy server list ping protocol
//! - Bedrock Edition via the RakNet Unconnected Ping protocol
//! - GS4 Query protocol for detailed server information
//!
//! ## Example (async)
//!
//! ```rust,ignore
//! use mcstatus::server::JavaServer;
//!
//! # async fn example() -> mcstatus::error::Result<()> {
//! let server = JavaServer::lookup("mc.example.com:25565", 3.0, false).await?;
//! let status = server.status().await?;
//! println!("MOTD: {}", status.motd.to_plain());
//! println!("Players: {}/{}", status.players.online, status.players.max);
//! # Ok(())
//! # }
//! ```

pub mod buffer;
pub mod error;
pub mod io;

// Core modules (built in later phases)
pub mod motd;
pub mod protocol;
pub mod response;
pub mod server;
pub mod address;
#[cfg(feature = "dns")]
pub mod dns;
pub mod connection;
pub mod util;
