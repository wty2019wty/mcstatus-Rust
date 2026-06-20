//! Server classes for querying Minecraft servers.
//!
//! Provides `JavaServer`, `BedrockServer`, and `LegacyServer` structs
//! that orchestrate address resolution, connection establishment, retry
//! logic, and protocol client usage.

use std::time::Duration;

use crate::address::Address;
use crate::connection::{TcpConnection, UdpConnection};
use crate::error::{McStatusError, Result};
use crate::protocol::bedrock::BedrockClient;
use crate::protocol::java::JavaClient;
use crate::protocol::legacy::LegacyClient;
use crate::protocol::query::QueryClient;
use crate::response::bedrock::BedrockStatusResponse;
use crate::response::java::JavaStatusResponse;
use crate::response::legacy::LegacyStatusResponse;
use crate::response::query::QueryResponse;

/// Default timeout in seconds.
const DEFAULT_TIMEOUT: f64 = 3.0;
/// Default number of retry attempts.
const DEFAULT_TRIES: usize = 3;

// ── JavaServer ───────────────────────────────────────────────────────────────

/// A Minecraft Java Edition server (1.7+).
#[derive(Debug, Clone)]
pub struct JavaServer {
    address: Address,
    timeout: Duration,
    query_port: u16,
}

impl JavaServer {
    /// Creates a new `JavaServer` instance.
    pub fn new(host: &str, port: Option<u16>, timeout: f64, query_port: Option<u16>) -> Result<Self> {
        let port = port.unwrap_or(25565);
        let query_port = query_port.unwrap_or(port);
        let address = Address::new(host.to_string(), port);
        let timeout = Duration::from_secs_f64(timeout);

        Ok(Self {
            address,
            timeout,
            query_port,
        })
    }

    /// Resolves the server address with SRV lookup, mimicking Minecraft's address field.
    #[cfg(feature = "dns")]
    pub async fn lookup(address: &str, timeout: f64) -> Result<Self> {
        let addr = crate::dns::minecraft_srv_address_lookup(address, 25565).await?;
        Ok(Self {
            address: addr,
            timeout: Duration::from_secs_f64(timeout),
            query_port: 25565,
        })
    }

    /// Resolves the server address without SRV lookup.
    #[cfg(not(feature = "dns"))]
    pub async fn lookup(address: &str, timeout: f64) -> Result<Self> {
        let addr = Address::parse_address(address, 25565)
            .map_err(|e| McStatusError::InvalidAddress(e))?;
        Ok(Self {
            address: addr,
            timeout: Duration::from_secs_f64(timeout),
            query_port: 25565,
        })
    }

    /// Returns the server's address.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Pings the server and returns the latency in milliseconds.
    pub async fn ping(&self) -> Result<f64> {
        self.ping_with_opts(DEFAULT_TRIES, 47, None).await
    }

    /// Pings the server with custom options.
    pub async fn ping_with_opts(
        &self,
        tries: usize,
        version: i32,
        ping_token: Option<i64>,
    ) -> Result<f64> {
        crate::util::retry(tries, || async {
            let conn = TcpConnection::connect(self.address.clone(), self.timeout).await?;
            let mut client = JavaClient::new(conn, self.address.clone(), version, ping_token);
            client.handshake().await?;
            client.test_ping().await
        })
        .await
    }

    /// Queries the server status via the status protocol.
    pub async fn status(&self) -> Result<JavaStatusResponse> {
        self.status_with_opts(DEFAULT_TRIES, 47, None).await
    }

    /// Queries the server status with custom options.
    pub async fn status_with_opts(
        &self,
        tries: usize,
        version: i32,
        ping_token: Option<i64>,
    ) -> Result<JavaStatusResponse> {
        crate::util::retry(tries, || async {
            let conn = TcpConnection::connect(self.address.clone(), self.timeout).await?;
            let mut client = JavaClient::new(conn, self.address.clone(), version, ping_token);
            client.handshake().await?;
            client.read_status().await
        })
        .await
    }

    /// Queries the server via the GS4 Query protocol.
    pub async fn query(&self) -> Result<QueryResponse> {
        self.query_with_opts(DEFAULT_TRIES).await
    }

    /// Queries the server via the Query protocol with custom retries.
    pub async fn query_with_opts(&self, tries: usize) -> Result<QueryResponse> {
        crate::util::retry(tries, || async {
            let ip = self
                .address
                .async_resolve_ip()
                .await
                .map_err(|e| McStatusError::Io(e))?;
            let query_addr = Address::new(ip.to_string(), self.query_port);
            let conn = UdpConnection::bind(query_addr, self.timeout).await?;
            let mut client = QueryClient::new(conn);
            client.handshake().await?;
            client.read_query().await
        })
        .await
    }
}

// ── BedrockServer ────────────────────────────────────────────────────────────

/// A Minecraft Bedrock Edition server.
#[derive(Debug, Clone)]
pub struct BedrockServer {
    address: Address,
    timeout: Duration,
}

impl BedrockServer {
    /// Creates a new `BedrockServer` instance.
    pub fn new(host: &str, port: Option<u16>, timeout: f64) -> Result<Self> {
        let port = port.unwrap_or(19132);
        let address = Address::new(host.to_string(), port);
        let timeout = Duration::from_secs_f64(timeout);

        Ok(Self { address, timeout })
    }

    /// Resolves the server address.
    pub async fn lookup(address: &str, timeout: f64) -> Result<Self> {
        let addr = Address::parse_address(address, 19132)
            .map_err(|e| McStatusError::InvalidAddress(e))?;
        Ok(Self {
            address: addr,
            timeout: Duration::from_secs_f64(timeout),
        })
    }

    /// Returns the server's address.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Queries the server status via the RakNet Unconnected Ping.
    pub async fn status(&self) -> Result<BedrockStatusResponse> {
        self.status_with_opts(DEFAULT_TRIES).await
    }

    /// Queries the server status with custom retries.
    pub async fn status_with_opts(&self, tries: usize) -> Result<BedrockStatusResponse> {
        crate::util::retry(tries, || async {
            let client = BedrockClient::new(self.address.clone(), self.timeout.as_secs_f64());
            client.read_status().await
        })
        .await
    }
}

// ── LegacyServer ─────────────────────────────────────────────────────────────

/// A legacy (pre-1.7) Minecraft Java Edition server.
#[derive(Debug, Clone)]
pub struct LegacyServer {
    address: Address,
    timeout: Duration,
}

impl LegacyServer {
    /// Creates a new `LegacyServer` instance.
    pub fn new(host: &str, port: Option<u16>, timeout: f64) -> Result<Self> {
        let port = port.unwrap_or(25565);
        let address = Address::new(host.to_string(), port);
        let timeout = Duration::from_secs_f64(timeout);

        Ok(Self { address, timeout })
    }

    /// Resolves the server address with SRV lookup.
    #[cfg(feature = "dns")]
    pub async fn lookup(address: &str, timeout: f64) -> Result<Self> {
        let addr = crate::dns::minecraft_srv_address_lookup(address, 25565).await?;
        Ok(Self {
            address: addr,
            timeout: Duration::from_secs_f64(timeout),
        })
    }

    /// Resolves the server address without SRV lookup.
    #[cfg(not(feature = "dns"))]
    pub async fn lookup(address: &str, timeout: f64) -> Result<Self> {
        let addr = Address::parse_address(address, 25565)
            .map_err(|e| McStatusError::InvalidAddress(e))?;
        Ok(Self {
            address: addr,
            timeout: Duration::from_secs_f64(timeout),
        })
    }

    /// Returns the server's address.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Queries the server status via the legacy protocol.
    pub async fn status(&self) -> Result<LegacyStatusResponse> {
        self.status_with_opts(DEFAULT_TRIES).await
    }

    /// Queries the server status with custom retries.
    pub async fn status_with_opts(&self, tries: usize) -> Result<LegacyStatusResponse> {
        crate::util::retry(tries, || async {
            let conn = TcpConnection::connect(self.address.clone(), self.timeout).await?;
            let mut client = LegacyClient::new(conn);
            client.read_status().await
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_server_new() {
        let server = JavaServer::new("example.com", Some(25565), 3.0, None).unwrap();
        assert_eq!(server.address().host, "example.com");
        assert_eq!(server.address().port, 25565);
    }

    #[test]
    fn test_java_server_default_port() {
        let server = JavaServer::new("example.com", None, 3.0, None).unwrap();
        assert_eq!(server.address().port, 25565);
    }

    #[test]
    fn test_bedrock_server_default_port() {
        let server = BedrockServer::new("example.com", None, 3.0).unwrap();
        assert_eq!(server.address().port, 19132);
    }

    #[test]
    fn test_legacy_server_default_port() {
        let server = LegacyServer::new("example.com", None, 3.0).unwrap();
        assert_eq!(server.address().port, 25565);
    }
}
