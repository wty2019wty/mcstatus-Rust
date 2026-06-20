//! Raw JSON response types for deserialization from server responses.
//!
//! These types mirror the JSON structure returned by Minecraft servers
//! and are used internally by the response builders.

use serde::Deserialize;

/// Raw Java Edition status response JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct RawJavaResponse {
    #[serde(default)]
    pub description: RawMotd,
    pub players: Option<RawJavaPlayers>,
    pub version: Option<RawJavaVersion>,
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: Option<bool>,
    #[serde(default)]
    pub favicon: Option<String>,
    #[serde(default)]
    #[serde(alias = "forgeData")]
    pub forge_data: Option<serde_json::Value>,
    /// Pre-1.18.1 forge data location
    #[serde(default)]
    pub modinfo: Option<RawForgeData>,
}

/// Raw MOTD value — can be a string or a JSON object.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RawMotd {
    String(String),
    Object(serde_json::Value),
}

impl Default for RawMotd {
    fn default() -> Self {
        RawMotd::String(String::new())
    }
}

/// Raw Java players section.
#[derive(Debug, Clone, Deserialize)]
pub struct RawJavaPlayers {
    pub online: u32,
    pub max: u32,
    #[serde(default)]
    pub sample: Option<Vec<RawJavaPlayer>>,
}

/// Raw Java player in the sample list.
#[derive(Debug, Clone, Deserialize)]
pub struct RawJavaPlayer {
    pub name: String,
    pub id: String,
}

/// Raw Java version section.
#[derive(Debug, Clone, Deserialize)]
pub struct RawJavaVersion {
    pub name: String,
    pub protocol: u32,
}

/// Raw query response (parsed from binary, not JSON).
#[derive(Debug, Clone, Default)]
pub struct RawQueryResponse {
    pub hostname: Option<String>,
    pub gametype: Option<String>,
    pub game_id: Option<String>,
    pub version: Option<String>,
    pub plugins: Option<String>,
    pub map: Option<String>,
    pub numplayers: Option<u32>,
    pub maxplayers: Option<u32>,
    pub hostport: Option<u16>,
    pub hostip: Option<String>,
    pub players: Vec<String>,
    pub software: Option<String>,
}

/// Raw forge data (pre-1.18.1 format or from `modinfo`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawForgeData {
    #[serde(default)]
    pub mods: Option<Vec<RawForgeMod>>,
    #[serde(default)]
    pub channels: Option<Vec<RawForgeChannel>>,
    #[serde(rename = "fmlNetworkVersion")]
    pub fml_network_version: Option<u32>,
    #[serde(default)]
    pub truncated: Option<bool>,
    /// Compressed data field (post-1.18.1 format)
    #[serde(default)]
    pub d: Option<String>,
}

/// Raw forge mod entry.
#[derive(Debug, Clone, Deserialize)]
pub struct RawForgeMod {
    #[serde(rename = "modId")]
    pub mod_id: Option<String>,
    #[serde(rename = "modmarker")]
    pub mod_marker: Option<String>,
    #[serde(rename = "modName")]
    pub mod_name: Option<String>,
    pub version: Option<String>,
}

/// Raw forge channel entry.
#[derive(Debug, Clone, Deserialize)]
pub struct RawForgeChannel {
    #[serde(rename = "res")]
    pub res: String,
    pub version: String,
    pub required: bool,
}
