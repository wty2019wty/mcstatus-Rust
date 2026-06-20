//! Bedrock Edition server status response model.

use serde::Serialize;

use crate::motd::Motd;

/// Response from a Bedrock Edition server status query.
#[derive(Debug, Clone, Serialize)]
pub struct BedrockStatusResponse {
    /// Player count information.
    pub players: BedrockStatusPlayers,
    /// Version information (includes server brand).
    pub version: BedrockStatusVersion,
    /// The parsed Message of the Day.
    pub motd: Motd,
    /// Round-trip latency in milliseconds.
    pub latency: f64,
    /// Current world/map name.
    pub map_name: Option<String>,
    /// Current gamemode.
    pub gamemode: Option<String>,
}

/// Player count for a Bedrock server.
#[derive(Debug, Clone, Serialize)]
pub struct BedrockStatusPlayers {
    pub online: u32,
    pub max: u32,
}

/// Version info for a Bedrock server.
#[derive(Debug, Clone, Serialize)]
pub struct BedrockStatusVersion {
    /// Version name (e.g. "1.20.80").
    pub name: Option<String>,
    /// Protocol version number.
    pub protocol: Option<u32>,
    /// Server brand (e.g. "Pocketmine-MP").
    pub brand: Option<String>,
}

impl BedrockStatusResponse {
    /// Builds a `BedrockStatusResponse` from parsed data fields.
    ///
    /// The `data` parameter should contain the semicolon-separated fields
    /// from the RakNet Unconnected Ping response, in order:
    /// `[brand, motd, protocol_version, version_name, online, max, game_mode_id, ...]`
    pub fn build(data: &[String], latency: f64) -> Result<Self, String> {
        if data.len() < 6 {
            return Err(format!(
                "Bedrock response has too few fields: expected at least 6, got {}",
                data.len()
            ));
        }

        let brand = if data[0].is_empty() {
            None
        } else {
            Some(data[0].clone())
        };

        let motd = Motd::from_string(&data[1], true);

        let protocol: Option<u32> = data[2].parse().ok();
        let version_name = if data[3].is_empty() {
            None
        } else {
            Some(data[3].clone())
        };

        let online: u32 = data[4].parse().unwrap_or(0);
        let max: u32 = data[5].parse().unwrap_or(0);

        let gamemode = data.get(7).and_then(|g| {
            if g.is_empty() {
                None
            } else {
                Some(g.clone())
            }
        });

        let map_name = data.get(8).and_then(|m| {
            if m.is_empty() {
                None
            } else {
                Some(m.clone())
            }
        });

        Ok(Self {
            players: BedrockStatusPlayers { online, max },
            version: BedrockStatusVersion {
                name: version_name,
                protocol,
                brand,
            },
            motd,
            latency,
            map_name,
            gamemode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_bedrock_response() {
        let data: Vec<String> = vec![
            "Pocketmine-MP",           // brand
            "§r§4G§r§6a§r§ey §r§2S§r§9e§r§1r§r§5v§r§ce§r§6r", // motd
            "422",                     // protocol version
            "1.18.100500",             // version name
            "0",                       // online players
            "20",                      // max players
            "Survival",                // game mode id
            "Creative",                // game mode name (unused by parser)
            "world",                   // map name
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let response = BedrockStatusResponse::build(&data, 42.0).unwrap();
        assert_eq!(response.players.online, 0);
        assert_eq!(response.players.max, 20);
        assert_eq!(response.version.protocol, Some(422));
        assert_eq!(response.version.name.as_deref(), Some("1.18.100500"));
        assert_eq!(response.version.brand.as_deref(), Some("Pocketmine-MP"));
        assert_eq!(response.map_name.as_deref(), Some("world"));
        assert_eq!(response.gamemode.as_deref(), Some("Creative"));
        assert_eq!(response.latency, 42.0);
    }

    #[test]
    fn test_build_minimal_bedrock() {
        let data: Vec<String> = vec!["Brand", "MOTD", "1", "v1", "0", "10"]
            .into_iter()
            .map(String::from)
            .collect();

        let response = BedrockStatusResponse::build(&data, 10.0).unwrap();
        assert_eq!(response.players.online, 0);
        assert_eq!(response.players.max, 10);
        assert_eq!(response.motd.to_plain(), "MOTD");
    }

    #[test]
    fn test_build_too_few_fields() {
        let data: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
        assert!(BedrockStatusResponse::build(&data, 0.0).is_err());
    }

    #[test]
    fn test_build_empty_fields() {
        let data: Vec<String> = vec![
            String::new(), // empty brand
            "Hello".into(),
            String::new(), // empty protocol
            String::new(), // empty version name
            "5".into(),
            "20".into(),
        ];

        let response = BedrockStatusResponse::build(&data, 0.0).unwrap();
        assert_eq!(response.version.brand, None);
        assert_eq!(response.version.protocol, None);
        assert_eq!(response.version.name, None);
    }
}
