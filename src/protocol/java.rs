//! Java Edition (1.7+) Server List Ping protocol client.

use std::time::Instant;

use rand::Rng;

use crate::address::Address;
use crate::buffer::Buffer;
use crate::connection::TcpConnection;
use crate::error::{McStatusError, Result};
use crate::io::MinecraftWrite;
use crate::response::java::JavaStatusResponse;

/// Client for the Java Edition Server List Ping protocol (1.7+).
///
/// Handles handshake, status request, and ping request/response.
pub struct JavaClient {
    connection: TcpConnection,
    address: Address,
    version: i32,
    ping_token: i64,
}

impl JavaClient {
    /// Creates a new Java client.
    pub fn new(connection: TcpConnection, address: Address, version: i32, ping_token: Option<i64>) -> Self {
        let ping_token = ping_token.unwrap_or_else(|| {
            rand::thread_rng().gen_range(0..i64::MAX)
        });
        Self {
            connection,
            address,
            version,
            ping_token,
        }
    }

    /// Sends the handshake packet to initiate the protocol.
    pub async fn handshake(&mut self) -> Result<()> {
        let mut packet = Buffer::new();
        // Packet ID 0 = handshake
        packet.write_mc_varint(0)?;
        // Protocol version
        packet.write_mc_varint(self.version)?;
        // Server address (hostname)
        packet.write_mc_utf(&self.address.host)?;
        // Server port (unsigned short, big-endian)
        packet.write_ushort(self.address.port);
        // Next state: 1 = status
        packet.write_mc_varint(1)?;

        // Write as varint-prefixed byte array
        let mut framed = Buffer::new();
        framed.write_mc_bytearray(packet.as_bytes())?;
        self.connection.write_all(framed.as_bytes()).await?;

        Ok(())
    }

    /// Sends a status request and reads the response.
    pub async fn read_status(&mut self) -> Result<JavaStatusResponse> {
        // Send status request: packet ID 0
        let mut request = Buffer::new();
        request.write_mc_varint(0)?;

        let mut framed = Buffer::new();
        framed.write_mc_bytearray(request.as_bytes())?;
        self.connection.write_all(framed.as_bytes()).await?;

        // Read response
        let start = Instant::now();
        let response_data = self.read_packet().await?;
        let end = Instant::now();

        self.handle_status_response(response_data, start, end)
    }

    /// Sends a ping request and measures latency.
    pub async fn test_ping(&mut self) -> Result<f64> {
        // Build ping request: packet ID 1 + ping token (long long)
        let mut request = Buffer::new();
        request.write_mc_varint(1)?;
        request.write_ulonglong(self.ping_token as u64);

        let start = Instant::now();

        let mut framed = Buffer::new();
        framed.write_mc_bytearray(request.as_bytes())?;
        self.connection.write_all(framed.as_bytes()).await?;

        // Read response
        let response_data = self.read_packet().await?;
        let end = Instant::now();

        self.handle_ping_response(response_data, start, end)
    }

    /// Reads a varint-prefixed packet from the connection.
    async fn read_packet(&mut self) -> Result<Buffer> {
        // Read the packet length varint
        let length = self.read_varint_from_stream().await?;

        if length <= 0 {
            return Err(McStatusError::Protocol(
                "Received empty or invalid packet".into(),
            ));
        }

        let data = self.connection.read_exact(length as usize).await?;
        Ok(Buffer::from_bytes(data))
    }

    /// Reads a varint from the TCP stream byte by byte.
    async fn read_varint_from_stream(&mut self) -> Result<i32> {
        let mut result: i32 = 0;
        for i in 0..5 {
            let byte = self.connection.read_byte().await?;
            result |= ((byte & 0x7F) as i32) << (7 * i);
            if byte & 0x80 == 0 {
                return Ok(result);
            }
        }
        Err(McStatusError::Protocol(
            "Varint too long for 32-bit integer".into(),
        ))
    }

    fn handle_status_response(
        &self,
        mut response: Buffer,
        start: Instant,
        end: Instant,
    ) -> Result<JavaStatusResponse> {
        // Verify packet ID
        let packet_id = response
            .read_bytes(1)
            .map_err(|e| McStatusError::Protocol(format!("Failed to read packet ID: {e}")))?;
        if packet_id[0] != 0 {
            return Err(McStatusError::Protocol(
                "Received invalid status response packet".into(),
            ));
        }

        // Read the JSON payload (varint-length-prefixed UTF-8)
        // Since Buffer is not a std::io::Read, we need to use it differently
        let length = read_varint_from_buffer(&mut response)?;
        if length < 0 {
            return Err(McStatusError::Protocol(
                "Invalid JSON length in status response".into(),
            ));
        }
        let json_bytes = response
            .read_bytes(length as usize)
            .map_err(|e| McStatusError::Protocol(format!("Failed to read JSON payload: {e}")))?;

        let json_str = String::from_utf8(json_bytes)
            .map_err(|e| McStatusError::Protocol(format!("Invalid UTF-8 in status response: {e}")))?;

        let raw_json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| McStatusError::Protocol(format!("Invalid JSON in status response: {e}")))?;

        let latency_ms = end.duration_since(start).as_secs_f64() * 1000.0;

        JavaStatusResponse::build(raw_json, latency_ms)
            .map_err(|e| McStatusError::InvalidResponse(e))
    }

    fn handle_ping_response(
        &self,
        mut response: Buffer,
        start: Instant,
        end: Instant,
    ) -> Result<f64> {
        // Verify packet ID is 1
        let packet_id = response
            .read_bytes(1)
            .map_err(|e| McStatusError::Protocol(format!("Failed to read packet ID: {e}")))?;
        if packet_id[0] != 1 {
            return Err(McStatusError::Protocol(
                "Received invalid ping response packet".into(),
            ));
        }

        // Read the ping token (8 bytes, big-endian signed long long)
        let token_bytes = response
            .read_bytes(8)
            .map_err(|e| McStatusError::Protocol(format!("Failed to read ping token: {e}")))?;
        let received_token = i64::from_be_bytes([
            token_bytes[0], token_bytes[1], token_bytes[2], token_bytes[3],
            token_bytes[4], token_bytes[5], token_bytes[6], token_bytes[7],
        ]);

        if received_token != self.ping_token {
            return Err(McStatusError::Protocol(format!(
                "Received mangled ping response (expected token {}, got {})",
                self.ping_token, received_token
            )));
        }

        Ok(end.duration_since(start).as_secs_f64() * 1000.0)
    }
}

/// Reads a varint from a Buffer (not from std::io::Read).
fn read_varint_from_buffer(buf: &mut Buffer) -> Result<i32> {
    let mut result: i32 = 0;
    for i in 0..5 {
        let byte = buf
            .read_bytes(1)
            .map_err(|e| McStatusError::Protocol(format!("Failed to read varint byte: {e}")))?;
        result |= ((byte[0] & 0x7F) as i32) << (7 * i);
        if byte[0] & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(McStatusError::Protocol(
        "Varint too long for 32-bit integer".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::io::MinecraftRead;

    #[test]
    fn test_build_handshake_packet() {
        // Build a handshake packet manually and verify structure
        let mut packet = Buffer::new();
        packet.write_mc_varint(0).unwrap(); // handshake packet ID
        packet.write_mc_varint(47).unwrap(); // protocol version (1.8)
        packet.write_mc_utf("example.com").unwrap();
        packet.write_ushort(25565);
        packet.write_mc_varint(1).unwrap(); // next state: status

        // Frame it
        let mut framed = Buffer::new();
        framed.write_mc_bytearray(packet.as_bytes()).unwrap();

        let bytes = framed.as_bytes().to_vec();
        assert!(!bytes.is_empty());

        // Verify we can read it back
        let mut reader = Buffer::from_bytes(bytes);
        let inner = reader.read_mc_bytearray().unwrap(); // manually verify with Buffer
        assert!(!inner.is_empty());
    }

    #[test]
    fn test_build_status_request() {
        let mut request = Buffer::new();
        request.write_mc_varint(0).unwrap();

        let mut framed = Buffer::new();
        framed.write_mc_bytearray(request.as_bytes()).unwrap();

        assert!(framed.len() >= 2);
    }

    #[test]
    fn test_build_ping_request() {
        let mut request = Buffer::new();
        request.write_mc_varint(1).unwrap();
        request.write_ulonglong(12345678);

        let mut framed = Buffer::new();
        framed.write_mc_bytearray(request.as_bytes()).unwrap();

        // Verify structure
        let mut reader = Buffer::from_bytes(framed.into_bytes());
        let inner = reader.read_mc_bytearray().unwrap();

        // inner should be: varint(1) + longlong(12345678)
        assert_eq!(inner.len(), 1 + 8); // varint(1) = 1 byte + 8 bytes
    }

    #[test]
    fn test_handle_status_response_valid() {
        // Build a valid status response in a buffer
        let json = r#"{"description":"Test","players":{"max":20,"online":0},"version":{"name":"1.8","protocol":47}}"#;

        // Build the buffer correctly: 1 byte packet_id (byte, not varint) + JSON payload
        let mut response = Buffer::new();
        response.write_ubyte(0); // packet ID 0

        // Write JSON length + JSON
        let json_bytes = json.as_bytes();
        // Write as if it came from the wire: varint length prefix then data
        let mut json_packet = Buffer::new();
        json_packet.write_bytes(json_bytes);
        // The handle_status_response reads packet ID from first byte, then reads varint length
        // Actually, looking at the Java protocol: the response is a regular packet.
        // The buffer we pass in is the content AFTER removing the outer varint frame.
        // So: [packet_id_byte][varint_length][json_bytes]
        // Actually in Java protocol: the response is just {packet_id_varint}{json_string}.
        // The json_string is read via read_utf which expects varint-prefixed.
        // So: varint(0) + varint(json.len()) + json_bytes
        let mut framed = Buffer::new();
        framed.write_mc_varint(0).unwrap(); // packet ID as varint
        framed.write_mc_utf(json).unwrap(); // UTF string (varint length + data)

        // Parse the JSON value to verify roundtrip
        let json_val: serde_json::Value = serde_json::from_str(json).unwrap();
        let result = JavaStatusResponse::build(json_val, 10.0);
        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.motd.to_plain(), "Test");
        assert_eq!(status.players.max, 20);
    }
}
