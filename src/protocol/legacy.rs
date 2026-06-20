//! Legacy (pre-1.7) Java Edition Server List Ping protocol client.

use std::time::Instant;

use crate::connection::TcpConnection;
use crate::error::{McStatusError, Result};
use crate::response::legacy::LegacyStatusResponse;

/// The magic bytes for the legacy server list ping request.
/// See: https://minecraft.wiki/w/Java_Edition_protocol/Server_List_Ping#Client_to_server
const REQUEST_STATUS_DATA: [u8; 3] = [0xFE, 0x01, 0xFA];

/// Client for the pre-1.7 Java Edition server list ping protocol.
pub struct LegacyClient {
    connection: TcpConnection,
}

impl LegacyClient {
    /// Creates a new Legacy client.
    pub fn new(connection: TcpConnection) -> Self {
        Self { connection }
    }

    /// Sends the legacy status request and reads the response.
    pub async fn read_status(&mut self) -> Result<LegacyStatusResponse> {
        let start = Instant::now();

        // Send the magic bytes
        self.connection.write_all(&REQUEST_STATUS_DATA).await?;

        // Read packet ID (should be 0xFF)
        let packet_id = self.connection.read_byte().await?;
        if packet_id != 0xFF {
            return Err(McStatusError::Protocol(
                "Received invalid packet ID from legacy server".into(),
            ));
        }

        // Read the length of the string data (unsigned short, big-endian)
        let length_bytes = self.connection.read_exact(2).await?;
        let length = u16::from_be_bytes([length_bytes[0], length_bytes[1]]) as usize;

        // Read the string data (UTF-16BE encoded, length is in characters, so 2 bytes per char)
        let data = self.connection.read_exact(length * 2).await?;
        let end = Instant::now();

        let latency = end.duration_since(start).as_secs_f64() * 1000.0;
        Self::parse_response(&data, latency)
    }

    /// Parses the legacy response data.
    fn parse_response(data: &[u8], latency: f64) -> Result<LegacyStatusResponse> {
        // Decode as UTF-16BE
        let decoded = decode_utf16be(data);
        let parts: Vec<&str> = decoded.split('\0').collect();

        if parts.len() < 1 {
            return Err(McStatusError::Protocol(
                "Received empty legacy response".into(),
            ));
        }

        let parsed = if parts[0] == "§1" {
            // 1.4+ format: §1\0<protocol>\0<version>\0<motd>\0<online>\0<max>
            if parts.len() < 6 {
                return Err(McStatusError::Protocol(
                    "Received invalid legacy response (expected 6 fields)".into(),
                ));
            }
            let protocol: u32 = parts[1].parse().unwrap_or(0);
            let version = parts[2].to_string();
            let motd = parts[3].to_string();
            let online: u32 = parts[4].parse().unwrap_or(0);
            let max: u32 = parts[5].parse().unwrap_or(0);
            LegacyStatusResponse::build(protocol, &version, &motd, online, max, latency)
        } else {
            // Pre-1.4 format: <motd>§<online>§<max>
            let legacy_parts: Vec<&str> = parts[0].split('§').collect();
            if legacy_parts.len() < 3 {
                return Err(McStatusError::Protocol(
                    "Received invalid kick packet reason".into(),
                ));
            }
            let motd = legacy_parts[0].to_string();
            let online: u32 = legacy_parts[1].parse().unwrap_or(0);
            let max: u32 = legacy_parts[2].parse().unwrap_or(0);
            LegacyStatusResponse::build(0, "<1.4", &motd, online, max, latency)
        };

        Ok(parsed)
    }
}

/// Decodes UTF-16BE bytes into a Rust String.
fn decode_utf16be(data: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i + 1 < data.len() {
        let code_unit = u16::from_be_bytes([data[i], data[i + 1]]);
        if let Some(ch) = char::from_u32(code_unit as u32) {
            result.push(ch);
        }
        i += 2;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_status_data() {
        assert_eq!(REQUEST_STATUS_DATA, [0xFE, 0x01, 0xFA]);
    }

    #[test]
    fn test_parse_response_1_4_format() {
        // Build a 1.4.7 response: §1\0<protocol>\0<version>\0<motd>\0<online>\0<max>
        let fields = ["§1", "61", "1.4.7", "A Minecraft Server", "5", "20"];
        let joined = fields.join("\0");

        // Encode as UTF-16BE
        let mut data = Vec::new();
        for ch in joined.encode_utf16() {
            data.extend_from_slice(&ch.to_be_bytes());
        }

        let result = LegacyClient::parse_response(&data, 12.3).unwrap();
        assert_eq!(result.version.protocol, 61);
        assert_eq!(result.version.name, "1.4.7");
        assert_eq!(result.motd.to_plain(), "A Minecraft Server");
        assert_eq!(result.players.online, 5);
        assert_eq!(result.players.max, 20);
        assert_eq!(result.latency, 12.3);
    }

    #[test]
    fn test_parse_response_pre_1_4_format() {
        // Pre-1.4 format: <motd>§<online>§<max>
        let data_str = "My Server§3§10";
        let mut data = Vec::new();
        for ch in data_str.encode_utf16() {
            data.extend_from_slice(&ch.to_be_bytes());
        }

        let result = LegacyClient::parse_response(&data, 5.0).unwrap();
        assert_eq!(result.version.name, "<1.4");
        assert_eq!(result.motd.to_plain(), "My Server");
        assert_eq!(result.players.online, 3);
        assert_eq!(result.players.max, 10);
    }
}
