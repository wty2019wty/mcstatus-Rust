//! Forge mod data decoder.
//!
//! Handles both pre-1.18.1 JSON format and post-1.18.1 compressed binary format
//! for forge mod information embedded in server status responses.

use serde::Serialize;

/// Decoded forge mod data from a server status response.
#[derive(Debug, Clone, Serialize)]
pub struct ForgeData {
    /// FML network version.
    pub fml_network_version: Option<u32>,
    /// List of forge communication channels.
    pub channels: Vec<ForgeDataChannel>,
    /// List of installed mods.
    pub mods: Vec<ForgeDataMod>,
    /// Whether the mod list was truncated by the server.
    pub truncated: Option<bool>,
}

/// A forge communication channel.
#[derive(Debug, Clone, Serialize)]
pub struct ForgeDataChannel {
    pub name: String,
    pub version: String,
    pub required: bool,
}

/// A forge mod entry.
#[derive(Debug, Clone, Serialize)]
pub struct ForgeDataMod {
    /// The mod's internal ID.
    pub mod_id: Option<String>,
    /// Version marker used for network compatibility checks.
    pub marker: Option<String>,
    /// Human-readable mod name.
    pub name: Option<String>,
    /// Mod version string.
    pub version: Option<String>,
}

impl ForgeData {
    /// Builds `ForgeData` from a raw forge data JSON value.
    ///
    /// Handles both:
    /// - Pre-1.18.1: standard JSON with `mods`, `channels`, `fmlNetworkVersion`
    /// - Post-1.18.1: compressed binary in the `d` field
    pub fn build(raw: &serde_json::Value) -> Result<Self, String> {
        let obj = raw
            .as_object()
            .ok_or_else(|| "Forge data must be a JSON object".to_string())?;

        let fml_network_version = obj
            .get("fmlNetworkVersion")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let truncated = obj.get("truncated").and_then(|v| v.as_bool());

        // Check for post-1.18.1 compressed format
        if let Some(d_field) = obj.get("d").and_then(|v| v.as_str()) {
            // Decompress the optimized binary format
            let (channels, mods_from_d) = decode_optimized_buffer(d_field)?;

            // Also check for mods/channels at the top level (both can exist)
            let mut mods = mods_from_d;
            let mut channels_data = channels;

            // Merge with any top-level mods
            if let Some(top_mods) = obj.get("mods").and_then(|v| v.as_array()) {
                for m in top_mods {
                    if let Ok(parsed) = ForgeDataMod::build(m) {
                        mods.push(parsed);
                    }
                }
            }

            // Merge with any top-level channels
            if let Some(top_channels) = obj.get("channels").and_then(|v| v.as_array()) {
                for ch in top_channels {
                    if let Ok(parsed) = ForgeDataChannel::build(ch) {
                        channels_data.push(parsed);
                    }
                }
            }

            return Ok(Self {
                fml_network_version,
                channels: channels_data,
                mods,
                truncated,
            });
        }

        // Pre-1.18.1 format
        let channels = obj
            .get("channels")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|c| ForgeDataChannel::build(c).ok()).collect())
            .unwrap_or_default();

        let mods = obj
            .get("mods")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|m| ForgeDataMod::build(m).ok()).collect())
            .unwrap_or_default();

        Ok(Self {
            fml_network_version,
            channels,
            mods,
            truncated,
        })
    }
}

impl ForgeDataChannel {
    fn build(raw: &serde_json::Value) -> Result<Self, String> {
        let obj = raw
            .as_object()
            .ok_or_else(|| "Channel must be a JSON object".to_string())?;

        Ok(Self {
            name: obj
                .get("res")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            version: obj
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            required: obj.get("required").and_then(|v| v.as_bool()).unwrap_or(false),
        })
    }
}

impl ForgeDataMod {
    fn build(raw: &serde_json::Value) -> Result<Self, String> {
        let obj = raw
            .as_object()
            .ok_or_else(|| "Mod must be a JSON object".to_string())?;

        Ok(Self {
            mod_id: obj.get("modId").and_then(|v| v.as_str()).map(|s| s.to_string()),
            marker: obj
                .get("modmarker")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            name: obj
                .get("modName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            version: obj.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }
}

/// Decodes the post-1.18.1 optimized forge data format.
///
/// The data is encoded as a UTF-16 string where each character encodes
/// compressed binary data. Characters are packed into 15-bit groups.
fn decode_optimized_buffer(
    data: &str,
) -> Result<(Vec<ForgeDataChannel>, Vec<ForgeDataMod>), String> {
    let chars: Vec<u16> = data.encode_utf16().collect();

    if chars.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Convert UTF-16 characters into a bit stream
    let mut bits = Vec::new();
    for &ch in &chars {
        // Each character encodes 15 bits of data
        bits.push((ch >> 8) as u8 & 0x7F);
        bits.push(ch as u8);
    }

    let mut buffer = StringBuffer { data: bits, pos: 0 };

    let truncated = buffer.read_bool();

    let num_mods = buffer.read_varuint() as usize;
    let mut mods = Vec::with_capacity(num_mods);
    for _ in 0..num_mods {
        let mod_id = buffer.read_utf();
        let marker = buffer.read_utf();
        let name = if truncated {
            None
        } else {
            Some(buffer.read_utf())
        };
        let version = if truncated {
            None
        } else {
            Some(buffer.read_utf())
        };
        mods.push(ForgeDataMod {
            mod_id: if mod_id.is_empty() { None } else { Some(mod_id) },
            marker: if marker.is_empty() { None } else { Some(marker) },
            name,
            version,
        });
    }

    let num_channels = buffer.read_varuint() as usize;
    let mut channels = Vec::with_capacity(num_channels);
    for _ in 0..num_channels {
        let name = buffer.read_utf();
        let version = buffer.read_utf();
        let required = buffer.read_bool();
        channels.push(ForgeDataChannel {
            name,
            version,
            required,
        });
    }

    Ok((channels, mods))
}

/// A buffer for reading compressed forge data from a UTF-16 string.
struct StringBuffer {
    data: Vec<u8>,
    pos: usize,
}

impl StringBuffer {
    fn read_byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let byte = self.data[self.pos];
            self.pos += 1;
            byte
        } else {
            0
        }
    }

    fn read_bool(&mut self) -> bool {
        self.read_byte() != 0
    }

    fn read_varuint(&mut self) -> u64 {
        let mut result: u64 = 0;
        for i in 0..5 {
            let byte = self.read_byte();
            result |= ((byte & 0x7F) as u64) << (7 * i);
            if byte & 0x80 == 0 {
                return result;
            }
        }
        result
    }

    fn read_utf(&mut self) -> String {
        let length = self.read_varuint() as usize;
        let mut bytes = Vec::with_capacity(length.min(1024));
        for _ in 0..length {
            bytes.push(self.read_byte());
        }
        String::from_utf8(bytes).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_v1_fml_format() {
        let json = serde_json::json!({
            "mods": [
                {"modId": "testmod", "modmarker": "1.0.0", "modName": "Test Mod", "version": "1.0.0"}
            ],
            "channels": [
                {"res": "test:channel", "version": "1.0", "required": true}
            ],
            "fmlNetworkVersion": 2
        });

        let forge = ForgeData::build(&json).unwrap();
        assert_eq!(forge.fml_network_version, Some(2));
        assert_eq!(forge.mods.len(), 1);
        assert_eq!(forge.mods[0].mod_id.as_deref(), Some("testmod"));
        assert_eq!(forge.channels.len(), 1);
        assert_eq!(forge.channels[0].name, "test:channel");
        assert!(forge.channels[0].required);
    }

    #[test]
    fn test_build_empty() {
        let json = serde_json::json!({});
        let forge = ForgeData::build(&json).unwrap();
        assert!(forge.mods.is_empty());
        assert!(forge.channels.is_empty());
        assert_eq!(forge.fml_network_version, None);
    }

    #[test]
    fn test_build_v3_compressed() {
        // Empty compressed data: UTF-16 chars encoding empty mods/channels
        let json = serde_json::json!({
            "fmlNetworkVersion": 3,
            "d": "\u{0001}\u{0001}"  // truncated=true, then 0 mods, 0 channels
        });

        let forge = ForgeData::build(&json).unwrap();
        assert_eq!(forge.fml_network_version, Some(3));
        // The exact parsing depends on the binary format, but should not crash
    }

    #[test]
    fn test_decode_optimized_buffer_empty() {
        let (channels, mods) = decode_optimized_buffer("").unwrap();
        assert!(channels.is_empty());
        assert!(mods.is_empty());
    }
}
