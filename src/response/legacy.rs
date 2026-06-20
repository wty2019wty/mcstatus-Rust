//! Legacy (pre-1.7) Java Edition server status response model.

use serde::Serialize;

use crate::motd::Motd;

/// Response from a legacy (pre-1.7) Java Edition server.
#[derive(Debug, Clone, Serialize)]
pub struct LegacyStatusResponse {
    /// Player count information.
    pub players: LegacyStatusPlayers,
    /// Version information.
    pub version: LegacyStatusVersion,
    /// The parsed Message of the Day.
    pub motd: Motd,
    /// Round-trip latency in milliseconds.
    pub latency: f64,
}

/// Player count for a legacy server.
#[derive(Debug, Clone, Serialize)]
pub struct LegacyStatusPlayers {
    pub online: u32,
    pub max: u32,
}

/// Version info for a legacy server.
#[derive(Debug, Clone, Serialize)]
pub struct LegacyStatusVersion {
    /// Version name (e.g. "1.4.7").
    pub name: String,
    /// Protocol version number.
    pub protocol: u32,
}

impl LegacyStatusResponse {
    /// Builds a `LegacyStatusResponse` from the parsed legacy ping fields.
    ///
    /// Parameters correspond to the legacy server list ping response format:
    /// `[protocol_version, version_name, motd, online_players, max_players]`
    pub fn build(
        protocol_version: u32,
        version_name: &str,
        motd_raw: &str,
        online_players: u32,
        max_players: u32,
        latency: f64,
    ) -> Self {
        Self {
            players: LegacyStatusPlayers {
                online: online_players,
                max: max_players,
            },
            version: LegacyStatusVersion {
                name: version_name.to_string(),
                protocol: protocol_version,
            },
            motd: Motd::from_string(motd_raw, false),
            latency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_legacy_response() {
        let response = LegacyStatusResponse::build(
            61,                // protocol version (1.4.7)
            "1.4.7",           // version name
            "§aA Minecraft Server", // MOTD
            5,                 // online players
            20,                // max players
            12.3,              // latency
        );

        assert_eq!(response.version.protocol, 61);
        assert_eq!(response.version.name, "1.4.7");
        assert_eq!(response.motd.to_plain(), "A Minecraft Server");
        assert_eq!(response.players.online, 5);
        assert_eq!(response.players.max, 20);
        assert_eq!(response.latency, 12.3);
    }

    #[test]
    fn test_build_legacy_empty_motd() {
        let response = LegacyStatusResponse::build(
            39,     // protocol (1.2.5)
            "1.2.5",
            "",
            0,
            0,
            0.0,
        );
        assert_eq!(response.motd.to_plain(), "");
    }
}
