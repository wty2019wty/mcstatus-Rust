//! Bedrock Edition RakNet Unconnected Ping protocol client.

use std::time::Instant;

use crate::address::Address;
use crate::connection::UdpConnection;
use crate::error::{McStatusError, Result};
use crate::response::bedrock::BedrockStatusResponse;

/// RakNet Unconnected Ping magic bytes.
/// See: https://minecraft.wiki/w/RakNet#Unconnected_Ping
const REQUEST_STATUS_DATA: [u8; 33] = [
    0x01, // Packet ID: Unconnected Ping
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Timestamp
    0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, // Magic bytes
    0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // ...
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Client GUID
];

/// Client for the Bedrock Edition RakNet Unconnected Ping protocol.
pub struct BedrockClient {
    address: Address,
    timeout: f64,
}

impl BedrockClient {
    /// Creates a new Bedrock client.
    pub fn new(address: Address, timeout: f64) -> Self {
        Self { address, timeout }
    }

    /// Queries the server status via the Unconnected Ping protocol.
    pub async fn read_status(&self) -> Result<BedrockStatusResponse> {
        let start = Instant::now();
        let data = self.read_status_raw().await?;
        let end = Instant::now();

        let latency = end.duration_since(start).as_secs_f64() * 1000.0;
        Self::parse_response(&data, latency)
    }

    async fn read_status_raw(&self) -> Result<Vec<u8>> {
        let timeout = std::time::Duration::from_secs_f64(self.timeout);
        let conn = UdpConnection::bind(self.address.clone(), timeout).await?;

        conn.send(REQUEST_STATUS_DATA).await?;
        let data = conn.recv(2048).await?;

        Ok(data)
    }

    /// Parses the raw RakNet Unconnected Ping response.
    fn parse_response(data: &[u8], latency: f64) -> Result<BedrockStatusResponse> {
        if data.len() < 35 {
            return Err(McStatusError::Protocol(
                "Bedrock response too short".into(),
            ));
        }

        // Skip first byte (packet ID)
        let data = &data[1..];

        // Read the server info string length at offset 32 (2 bytes, big-endian)
        if data.len() < 34 {
            return Err(McStatusError::Protocol(
                "Bedrock response too short for name length".into(),
            ));
        }

        let name_length = u16::from_be_bytes([data[32], data[33]]) as usize;

        if data.len() < 34 + name_length {
            return Err(McStatusError::Protocol(
                "Bedrock response too short for server info".into(),
            ));
        }

        let name_data = &data[34..34 + name_length];

        let name_str = String::from_utf8_lossy(name_data);
        let fields: Vec<String> = name_str.split(';').map(|s| s.to_string()).collect();

        BedrockStatusResponse::build(&fields, latency)
            .map_err(|e| McStatusError::Protocol(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_status_data() {
        // Verify the magic bytes are correct
        assert_eq!(REQUEST_STATUS_DATA.len(), 33);
        assert_eq!(REQUEST_STATUS_DATA[0], 0x01); // Packet ID
    }

    #[test]
    fn test_parse_response_valid() {
        // Build a minimal valid Bedrock response
        let mut data = vec![0u8; 34];
        // Add server info string (semicolon-delimited fields)
        let info = "Pocketmine-MP;§cHello;422;1.18.0;0;20;Survival;;world";
        let info_bytes = info.as_bytes();
        let name_length = info_bytes.len() as u16;

        data.extend_from_slice(&name_length.to_be_bytes());
        data.push(0x1c); // packet ID byte (at position 0 in original data)

        // Now build full response: packet_id + padding + name_length + info
        let mut response = vec![0x1c]; // packet ID
        response.extend_from_slice(&vec![0u8; 33]); // padding
        response.extend_from_slice(&name_length.to_be_bytes());
        response.extend_from_slice(info_bytes);

        let result = BedrockClient::parse_response(&response, 42.0);
        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.players.online, 0);
        assert_eq!(status.players.max, 20);
        assert_eq!(status.latency, 42.0);
    }

    #[test]
    fn test_parse_response_too_short() {
        let data = vec![0u8; 10];
        assert!(BedrockClient::parse_response(&data, 0.0).is_err());
    }
}
