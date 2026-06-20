//! GS4 Query protocol response model.

use serde::Serialize;

use crate::motd::Motd;
use super::raw::RawQueryResponse;

/// Response from a GS4 Query protocol query.
///
/// The query protocol provides more detailed information than the status
/// protocol, including full player lists, plugin lists, and map/game info.
#[derive(Debug, Clone, Serialize)]
pub struct QueryResponse {
    /// The raw parsed query response.
    pub raw: RawQueryResponse,
    /// The parsed Message of the Day.
    pub motd: Motd,
    /// The current world/map name.
    pub map_name: Option<String>,
    /// Server IP address (as reported by the server).
    pub ip: Option<String>,
    /// Server port (as reported by the server).
    pub port: Option<u16>,
    /// Game type (e.g. "SMP").
    pub game_type: Option<String>,
    /// Game type ID.
    pub game_id: Option<String>,
    /// Player information.
    pub players: QueryPlayers,
    /// Software/plugin information.
    pub software: Option<QuerySoftware>,
}

/// Player information from a query response.
#[derive(Debug, Clone, Serialize)]
pub struct QueryPlayers {
    /// Number of players currently online.
    pub online: u32,
    /// Maximum number of players allowed.
    pub max: u32,
    /// List of online player names.
    pub list: Vec<String>,
}

/// Software and plugin information from a query response.
#[derive(Debug, Clone, Serialize)]
pub struct QuerySoftware {
    /// Server software version (e.g. "1.21.4").
    pub version: Option<String>,
    /// Server software brand (e.g. "Paper on 1.21.4").
    pub brand: Option<String>,
    /// List of installed plugins.
    pub plugins: Vec<String>,
}

impl QueryResponse {
    /// Builds a `QueryResponse` from a raw parsed query response and player list.
    pub fn build(raw: RawQueryResponse, players: Vec<String>) -> Self {
        let raw = raw;
        let motd_raw = raw.hostname.as_deref().unwrap_or("");
        let motd = Motd::from_string(motd_raw, false);

        // Parse plugins from the plugins field
        let (brand, plugins) = raw
            .plugins
            .as_deref()
            .map(parse_plugins)
            .unwrap_or((None, Vec::new()));

        let software = if brand.is_some() || !plugins.is_empty() || raw.software.is_some() {
            Some(QuerySoftware {
                version: raw.software.clone(),
                brand,
                plugins,
            })
        } else {
            None
        };

        let player_list = if !players.is_empty() { players } else { raw.players.clone() };

        Self {
            motd,
            map_name: raw.map.clone(),
            ip: raw.hostip.clone(),
            port: raw.hostport,
            game_type: raw.gametype.clone(),
            game_id: raw.game_id.clone(),
            players: QueryPlayers {
                online: raw.numplayers.unwrap_or(0),
                max: raw.maxplayers.unwrap_or(0),
                list: player_list,
            },
            software,
            raw,
        }
    }
}

/// Parses the plugins string from a query response.
///
/// Format: `"brand: plugin1; plugin2; plugin3"` (semicolon-separated)
/// The first element before `:` is the server brand.
fn parse_plugins(plugins_str: &str) -> (Option<String>, Vec<String>) {
    if let Some(colon_pos) = plugins_str.find(':') {
        let brand = plugins_str[..colon_pos].trim().to_string();
        let brand = if brand.is_empty() {
            None
        } else {
            Some(brand)
        };

        let plugin_list = plugins_str[colon_pos + 1..]
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        (brand, plugin_list)
    } else {
        (Some(plugins_str.to_string()), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query_response() {
        let raw = RawQueryResponse {
            hostname: Some("A Minecraft Server".into()),
            gametype: Some("SMP".into()),
            game_id: Some("MINECRAFT".into()),
            version: Some("1.21.4".into()),
            plugins: Some("Paper on 1.21.4: Essentials; WorldEdit".into()),
            map: Some("world".into()),
            numplayers: Some(5),
            maxplayers: Some(20),
            hostport: Some(25565),
            hostip: Some("192.168.1.1".into()),
            players: vec!["Player1".into(), "Player2".into()],
            software: None,
        };

        let response = QueryResponse::build(raw, vec![]);
        assert_eq!(response.motd.to_plain(), "A Minecraft Server");
        assert_eq!(response.players.online, 5);
        assert_eq!(response.players.max, 20);
        assert_eq!(response.players.list.len(), 2); // uses raw.players when passed list is empty
        assert_eq!(response.map_name.as_deref(), Some("world"));
        assert_eq!(response.game_type.as_deref(), Some("SMP"));

        let sw = response.software.unwrap();
        assert_eq!(sw.brand.as_deref(), Some("Paper on 1.21.4"));
        assert_eq!(sw.plugins.len(), 2);
        assert_eq!(sw.plugins[0], "Essentials");
    }

    #[test]
    fn test_build_minimal_query() {
        let raw = RawQueryResponse::default();
        let response = QueryResponse::build(raw, vec![]);
        assert_eq!(response.motd.to_plain(), "");
        assert_eq!(response.players.online, 0);
        assert_eq!(response.players.max, 0);
    }

    #[test]
    fn test_parse_plugins_no_colon() {
        let (brand, plugins) = parse_plugins("Vanilla");
        assert_eq!(brand.as_deref(), Some("Vanilla"));
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_parse_plugins_empty_brand() {
        let (brand, plugins) = parse_plugins(": plugin1; plugin2");
        assert_eq!(brand, None);
        assert_eq!(plugins.len(), 2);
    }
}
