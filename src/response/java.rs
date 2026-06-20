//! Java Edition (1.7+) server status response models.

use serde::Serialize;

use crate::motd::Motd;
use super::forge::ForgeData;
use super::raw::{RawJavaResponse, RawMotd};

/// Response from a Java Edition server status query.
#[derive(Debug, Clone, Serialize)]
pub struct JavaStatusResponse {
    /// The raw JSON response from the server.
    pub raw: serde_json::Value,
    /// Player count information.
    pub players: JavaStatusPlayers,
    /// Version information.
    pub version: JavaStatusVersion,
    /// The parsed Message of the Day.
    pub motd: Motd,
    /// Round-trip latency in milliseconds.
    pub latency: f64,
    /// Whether the server enforces secure chat (1.19+).
    pub enforces_secure_chat: Option<bool>,
    /// Base64-encoded PNG server icon.
    pub icon: Option<String>,
    /// Forge mod data, if available.
    pub forge_data: Option<ForgeData>,
}

/// Player count and sample information for a Java Edition server.
#[derive(Debug, Clone, Serialize)]
pub struct JavaStatusPlayers {
    /// Number of players currently online.
    pub online: u32,
    /// Maximum number of players allowed.
    pub max: u32,
    /// Optional sample of online player names and UUIDs.
    pub sample: Option<Vec<JavaStatusPlayer>>,
}

/// A player in the sample list.
#[derive(Debug, Clone, Serialize)]
pub struct JavaStatusPlayer {
    /// Player's display name.
    pub name: String,
    /// Player's UUID (as a string).
    pub id: String,
}

/// Version information for a Java Edition server.
#[derive(Debug, Clone, Serialize)]
pub struct JavaStatusVersion {
    /// Version name (e.g. "1.21.4").
    pub name: String,
    /// Protocol version number.
    pub protocol: u32,
}

impl JavaStatusResponse {
    /// Builds a `JavaStatusResponse` from raw JSON and a latency measurement.
    pub fn build(raw_json: serde_json::Value, latency: f64) -> Result<Self, String> {
        let raw: RawJavaResponse = serde_json::from_value(raw_json.clone())
            .map_err(|e| format!("Failed to parse Java status JSON: {e}"))?;

        let version = raw.version.map_or_else(
            || Err("Missing 'version' field in status response".to_string()),
            |v| Ok(JavaStatusVersion {
                name: v.name,
                protocol: v.protocol,
            }),
        )?;

        let players = raw.players.map_or(
            JavaStatusPlayers {
                online: 0,
                max: 0,
                sample: None,
            },
            |p| JavaStatusPlayers {
                online: p.online,
                max: p.max,
                sample: p.sample.map(|sample| {
                    sample
                        .into_iter()
                        .map(|s| JavaStatusPlayer {
                            name: s.name,
                            id: s.id,
                        })
                        .collect()
                }),
            },
        );

        let motd = match &raw.description {
            RawMotd::String(s) => Motd::from_string(s, false),
            RawMotd::Object(v) => Motd::from_json(v, false),
        };

        let forge_data = raw
            .forge_data
            .or(raw.modinfo.map(|m| serde_json::to_value(m).unwrap_or_default()))
            .and_then(|fd| ForgeData::build(&fd).ok());

        Ok(Self {
            raw: raw_json,
            players,
            version,
            motd,
            latency,
            enforces_secure_chat: raw.enforces_secure_chat,
            icon: raw.favicon,
            forge_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_minimal_response() {
        let json = serde_json::json!({
            "description": "A Minecraft Server",
            "players": {
                "max": 20,
                "online": 0
            },
            "version": {
                "name": "1.8-pre1",
                "protocol": 44
            }
        });

        let response = JavaStatusResponse::build(json, 15.5).unwrap();
        assert_eq!(response.players.max, 20);
        assert_eq!(response.players.online, 0);
        assert_eq!(response.version.name, "1.8-pre1");
        assert_eq!(response.version.protocol, 44);
        assert_eq!(response.latency, 15.5);
        assert_eq!(response.motd.to_plain(), "A Minecraft Server");
    }

    #[test]
    fn test_build_with_sample() {
        let json = serde_json::json!({
            "description": "Test",
            "players": {
                "max": 10,
                "online": 2,
                "sample": [
                    {"name": "Player1", "id": "uuid-1"},
                    {"name": "Player2", "id": "uuid-2"}
                ]
            },
            "version": {
                "name": "1.21",
                "protocol": 766
            }
        });

        let response = JavaStatusResponse::build(json, 10.0).unwrap();
        let sample = response.players.sample.unwrap();
        assert_eq!(sample.len(), 2);
        assert_eq!(sample[0].name, "Player1");
        assert_eq!(sample[1].id, "uuid-2");
    }

    #[test]
    fn test_build_missing_version() {
        let json = serde_json::json!({
            "description": "No version field"
        });
        assert!(JavaStatusResponse::build(json, 0.0).is_err());
    }

    #[test]
    fn test_build_missing_description() {
        let json = serde_json::json!({
            "version": {"name": "1.8", "protocol": 47}
        });
        // Should succeed — description defaults to empty string
        let response = JavaStatusResponse::build(json, 0.0).unwrap();
        assert_eq!(response.motd.to_plain(), "");
    }

    #[test]
    fn test_build_with_secure_chat_and_icon() {
        let json = serde_json::json!({
            "description": "Secure Server",
            "players": {"max": 100, "online": 50},
            "version": {"name": "1.19.2", "protocol": 760},
            "enforcesSecureChat": true,
            "favicon": "data:image/png;base64,abc123"
        });

        let response = JavaStatusResponse::build(json, 5.0).unwrap();
        assert_eq!(response.enforces_secure_chat, Some(true));
        assert_eq!(response.icon, Some("data:image/png;base64,abc123".into()));
    }
}
