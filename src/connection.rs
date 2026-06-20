//! TCP and UDP socket connections with Minecraft protocol I/O.
//!
//! Provides async (tokio-based) connections that implement the Minecraft
//! protocol read/write traits.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

use crate::address::Address;
use crate::error::{McStatusError, Result};

/// A TCP connection to a Minecraft server.
///
/// Wraps a tokio `TcpStream` and provides sync-like read/write
/// via the `std::io::Read` and `std::io::Write` traits through
/// internal buffering.
pub struct TcpConnection {
    stream: TcpStream,
    address: Address,
    timeout: Duration,
}

impl TcpConnection {
    /// Establishes a new TCP connection to the given address.
    pub async fn connect(address: Address, timeout: Duration) -> Result<Self> {
        let socket_addr = resolve_socket_addr(&address).await?;

        let stream = tokio::time::timeout(timeout, TcpStream::connect(socket_addr))
            .await
            .map_err(|_| McStatusError::Timeout)?
            .map_err(|e| McStatusError::Io(e))?;

        // Set TCP_NODELAY for lower latency
        stream
            .set_nodelay(true)
            .map_err(|e| McStatusError::Io(e))?;

        Ok(Self {
            stream,
            address,
            timeout,
        })
    }

    /// Returns the address this connection is connected to.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Reads exactly `length` bytes from the connection.
    pub async fn read_exact(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];
        tokio::time::timeout(self.timeout, self.stream.read_exact(&mut buf))
            .await
            .map_err(|_| McStatusError::Timeout)?
            .map_err(|e| McStatusError::Io(e))?;
        Ok(buf)
    }

    /// Writes data to the connection.
    pub async fn write_all(&mut self, data: &[u8]) -> Result<()> {
        tokio::time::timeout(self.timeout, self.stream.write_all(data))
            .await
            .map_err(|_| McStatusError::Timeout)?
            .map_err(|e| McStatusError::Io(e))?;
        Ok(())
    }

    /// Reads data until the connection is closed or a minimum amount is read.
    pub async fn read_to_end(&mut self, max_size: usize) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(max_size.min(65536));
        let mut temp = [0u8; 4096];

        loop {
            let n = tokio::time::timeout(self.timeout, self.stream.read(&mut temp))
                .await
                .map_err(|_| McStatusError::Timeout)?
                .map_err(|e| McStatusError::Io(e))?;

            if n == 0 {
                break; // EOF
            }
            buf.extend_from_slice(&temp[..n]);
            if buf.len() >= max_size {
                break;
            }
        }

        Ok(buf)
    }

    /// Reads a single byte from the connection.
    pub async fn read_byte(&mut self) -> Result<u8> {
        let bytes = self.read_exact(1).await?;
        Ok(bytes[0])
    }

    /// Writes a single byte to the connection.
    pub async fn write_byte(&mut self, byte: u8) -> Result<()> {
        self.write_all(&[byte]).await
    }
}

/// A UDP connection for querying Minecraft servers (Query protocol and Bedrock).
pub struct UdpConnection {
    socket: UdpSocket,
    address: Address,
    timeout: Duration,
}

impl UdpConnection {
    /// Creates a new UDP "connection" bound to an ephemeral port.
    pub async fn bind(address: Address, timeout: Duration) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| McStatusError::Io(e))?;

        Ok(Self {
            socket,
            address,
            timeout,
        })
    }

    /// Returns the address this connection targets.
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Sends data to the target address.
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let socket_addr = resolve_socket_addr(&self.address).await?;

        tokio::time::timeout(self.timeout, self.socket.send_to(data, socket_addr))
            .await
            .map_err(|_| McStatusError::Timeout)?
            .map_err(|e| McStatusError::Io(e))?;

        Ok(())
    }

    /// Receives data from the target (up to `max_size` bytes).
    pub async fn recv(&self, max_size: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; max_size];

        let (n, _addr) = tokio::time::timeout(self.timeout, self.socket.recv_from(&mut buf))
            .await
            .map_err(|_| McStatusError::Timeout)?
            .map_err(|e| McStatusError::Io(e))?;

        buf.truncate(n);
        Ok(buf)
    }
}

/// Helper to resolve an Address into a SocketAddr.
async fn resolve_socket_addr(address: &Address) -> Result<SocketAddr> {
    let ip = address
        .async_resolve_ip()
        .await
        .map_err(|e| McStatusError::Io(e))?;
    Ok(SocketAddr::new(ip, address.port))
}

// For backwards-compat with sync code in tests

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_connection_timeout() {
        // Try connecting to a non-routable address — should timeout
        let addr = Address::new("10.255.255.1".into(), 25565);
        let result = TcpConnection::connect(addr, Duration::from_millis(100)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_udp_connection_bind() {
        let addr = Address::new("127.0.0.1".into(), 25565);
        let conn = UdpConnection::bind(addr, Duration::from_secs(1)).await.unwrap();
        assert_eq!(conn.address().port, 25565);
    }
}
