//! GS4 Query protocol client.
//!
//! The query protocol uses UDP and requires a challenge-response handshake
//! before the full query can be performed.

use rand::Rng;

use crate::connection::UdpConnection;
use crate::error::{McStatusError, Result};
use crate::response::query::QueryResponse;
use crate::response::raw::RawQueryResponse;

/// Magic prefix for all query packets.
const MAGIC_PREFIX: [u8; 2] = [0xFE, 0xFD];
/// Packet type for challenge request.
const PACKET_TYPE_CHALLENGE: u8 = 9;
/// Packet type for query request.
const PACKET_TYPE_QUERY: u8 = 0;
/// Padding bytes for query packets.
const PADDING: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// Client for the GS4 Query protocol.
pub struct QueryClient {
    connection: UdpConnection,
    challenge: i32,
}

impl QueryClient {
    /// Creates a new Query client.
    pub fn new(connection: UdpConnection) -> Self {
        Self {
            connection,
            challenge: 0,
        }
    }

    /// Generates a session ID (lower 4 bits only, per Minecraft spec).
    fn generate_session_id() -> i32 {
        let raw: i32 = rand::thread_rng().gen_range(0..i32::MAX);
        raw & 0x0F0F_0F0F
    }

    /// Performs the challenge handshake to get a challenge token.
    pub async fn handshake(&mut self) -> Result<()> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&MAGIC_PREFIX);
        packet.push(PACKET_TYPE_CHALLENGE);
        packet.extend_from_slice(&Self::generate_session_id().to_be_bytes());

        self.connection.send(&packet).await?;
        let response = self.connection.recv(4096).await?;

        // Response format: 0x09 + session_id(4 bytes) + null-terminated challenge string
        if response.len() < 6 {
            return Err(McStatusError::Protocol(
                "Query handshake response too short".into(),
            ));
        }

        // Skip type (1 byte) and session ID (4 bytes), read challenge string
        let challenge_bytes = &response[5..];
        let challenge_str = read_null_terminated_ascii(challenge_bytes)?;
        self.challenge = challenge_str
            .parse()
            .map_err(|_| McStatusError::Protocol("Invalid challenge token".into()))?;

        Ok(())
    }

    /// Sends a full query request and reads the response.
    pub async fn read_query(&mut self) -> Result<QueryResponse> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&MAGIC_PREFIX);
        packet.push(PACKET_TYPE_QUERY);
        packet.extend_from_slice(&Self::generate_session_id().to_be_bytes());
        packet.extend_from_slice(&self.challenge.to_be_bytes());
        packet.extend_from_slice(&PADDING);

        self.connection.send(&packet).await?;
        let mut response = self.connection.recv(65536).await?;

        // Parse response
        let (raw, players) = Self::parse_response(&mut response)?;
        Ok(QueryResponse::build(raw, players))
    }

    /// Parses the raw query response into structured data.
    fn parse_response(
        data: &mut [u8],
    ) -> Result<(RawQueryResponse, Vec<String>)> {
        // Response format:
        // 0x00 (type) + session_id (4 bytes) + token (4 bytes)
        // + "splitnum\0" + 128 (padding byte) + 0x00
        // Then key\0value\0 pairs, ending with \0\0
        // Then "\x01player_\0" + player\0player\0... + \0

        let offset = 1 + 4 + 4; // Skip type + session_id + token
        if data.len() < offset + 13 {
            return Err(McStatusError::Protocol(
                "Query response too short".into(),
            ));
        }

        let body = &mut data[offset..];

        // Skip "splitnum\0" + padding
        let mut pos = 0;
        while pos < body.len() && body[pos] != 0x00 {
            pos += 1;
        }
        pos += 1; // skip null
        if pos < body.len() && body[pos] == 0x80 {
            pos += 1;
        }
        if pos < body.len() && body[pos] == 0x00 {
            pos += 1;
        }

        // Read key-value pairs
        let mut raw = RawQueryResponse::default();

        loop {
            if pos >= body.len() {
                break;
            }

            let key = read_null_terminated_iso(&body[pos..])?;
            pos += key.len() + 1; // +1 for null terminator

            if key.is_empty() {
                // Empty key signals end of KV section (actually it's another byte to skip)
                if pos < body.len() && body[pos] == 0x00 {
                    pos += 1;
                }
                break;
            }

            if key == "hostname" {
                // hostname (MOTD) is special — it may contain null bytes within it.
                // We need to find where it ends by looking for the next known key.
                let known_keys = [
                    "hostip", "hostport", "game_id", "gametype", "map",
                    "maxplayers", "numplayers", "plugins", "version",
                ];

                let mut found_key_idx = None;
                for k in &known_keys {
                    // Search for "\0{key}\0" pattern — the \0 before the key marks end of MOTD
                    let pattern = format!("\0{k}\0");
                    if let Some(idx) = find_subsequence(&body[pos..], pattern.as_bytes()) {
                        if found_key_idx.map_or(true, |prev| idx < prev) {
                            found_key_idx = Some(idx);
                        }
                    }
                }

                let motd_end = if let Some(idx) = found_key_idx {
                    pos + idx
                } else {
                    // Fallback: read until double null
                    let mut e = pos;
                    while e < body.len() - 1 && !(body[e] == 0x00 && body[e + 1] == 0x00) {
                        e += 1;
                    }
                    e
                };

                if motd_end > pos {
                    raw.hostname = Some(
                        std::str::from_utf8(&body[pos..motd_end])
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                pos = motd_end;
                // Skip the trailing null
                if pos < body.len() && body[pos] == 0x00 {
                    pos += 1;
                }
            } else {
                let value = read_null_terminated_iso(&body[pos..])?;
                pos += value.len() + 1;

                match key.as_str() {
                    "numplayers" => raw.numplayers = value.parse().ok(),
                    "maxplayers" => raw.maxplayers = value.parse().ok(),
                    "hostport" => raw.hostport = value.parse().ok(),
                    "hostip" => raw.hostip = Some(value),
                    "gametype" => raw.gametype = Some(value),
                    "game_id" => raw.game_id = Some(value),
                    "map" => raw.map = Some(value),
                    "version" => raw.version = Some(value),
                    "plugins" => raw.plugins = Some(value),
                    _ => { /* skip unknown keys */ }
                }
            }
        }

        // Read players section
        // Skip "\x01player_\0" marker
        let mut players = Vec::new();
        while pos < body.len() {
            if body[pos] == 0x00 {
                pos += 1;
                continue;
            }
            if body[pos] == 0x01 {
                pos += 1;
                // Read "player_\0" marker
                let marker = read_null_terminated_iso(&body[pos..])?;
                if marker == "player_" {
                    pos += marker.len() + 1;
                    // Now read player names
                    while pos < body.len() {
                        let player = read_null_terminated_iso(&body[pos..])?;
                        pos += player.len() + 1;
                        if player.is_empty() {
                            break;
                        }
                        players.push(player);
                    }
                    break;
                }
            }
            if body[pos] == 0x00 {
                pos += 1;
            } else {
                break;
            }
        }

        Ok((raw, players))
    }
}

/// Reads a null-terminated string encoded as ISO-8859-1 (Latin-1).
fn read_null_terminated_iso(data: &[u8]) -> Result<String> {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let bytes = &data[..end];
    Ok(bytes.iter().map(|&b| b as char).collect())
}

/// Reads a null-terminated ASCII string from bytes.
fn read_null_terminated_ascii(data: &[u8]) -> Result<String> {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    Ok(String::from_utf8_lossy(&data[..end]).to_string())
}

/// Finds a subsequence within a byte slice, returning the index.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_id() {
        let id = QueryClient::generate_session_id();
        // Upper 4 bits of each byte should be 0 (masked by 0x0F0F0F0F)
        assert_eq!(id & !0x0F0F_0F0F, 0);
    }

    #[test]
    fn test_read_null_terminated_iso() {
        let data = b"hello\0world";
        let result = read_null_terminated_iso(data).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_parse_query_response() {
        // Build a minimal valid query response
        let mut data = Vec::new();
        // Header: type(0x00) + session_id(4 bytes) + token(4 bytes)
        data.push(0x00); // type
        data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]); // session_id
        data.extend_from_slice(&[0x05, 0x06, 0x07, 0x08]); // token (challenge)
        // "splitnum\0" + padding
        data.extend_from_slice(b"splitnum\0\x80\x00");

        // Key-value pairs
        data.extend_from_slice(b"hostname\0Test Server\0");
        data.extend_from_slice(b"numplayers\05\0");
        data.extend_from_slice(b"maxplayers\020\0");
        data.extend_from_slice(b"gametype\0SMP\0");

        // End of KV section
        data.push(0x00);
        data.push(0x00);

        // Players section
        data.push(0x01);
        data.extend_from_slice(b"player_\0");
        data.extend_from_slice(b"Player1\0");
        data.extend_from_slice(b"Player2\0");
        data.push(0x00);

        let (raw, players) = QueryClient::parse_response(&mut data).unwrap();
        assert_eq!(raw.hostname.as_deref(), Some("Test Server"));
        assert_eq!(raw.numplayers, Some(5));
        assert_eq!(raw.maxplayers, Some(20));
        assert_eq!(players.len(), 2);
        assert_eq!(players[0], "Player1");
    }
}
