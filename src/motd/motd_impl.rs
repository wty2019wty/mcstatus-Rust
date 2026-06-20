//! The Motd struct — parses raw MOTD data and provides transformation methods.

use serde_json::Value;

use super::components::{Formatting, MinecraftColor, MotdComponent, WebColor};
use super::simplify::{simplify, ComponentList};
use super::transform;

/// A parsed Message of the Day from a Minecraft server.
///
/// Can be constructed from either:
/// - A legacy string with `§` or `&` color codes
/// - A modern JSON chat component dict
#[derive(Debug, Clone, serde::Serialize)]
pub struct Motd {
    /// The parsed component list.
    #[serde(skip)]
    pub parsed: ComponentList,
    /// The raw input (for round-tripping).
    pub raw: MotdRaw,
    /// Whether this was parsed from a Bedrock server response.
    pub bedrock: bool,
}

/// The raw MOTD input type.
#[derive(Debug, Clone, serde::Serialize)]
pub enum MotdRaw {
    /// A legacy string with §/& codes.
    String(String),
    /// A modern JSON chat component.
    Json(serde_json::Value),
}

impl Motd {
    /// Parses a raw MOTD string (with § or & color codes).
    pub fn from_string(raw: &str, bedrock: bool) -> Self {
        let parsed = parse_legacy_string(raw);
        Self {
            parsed,
            raw: MotdRaw::String(raw.to_string()),
            bedrock,
        }
    }

    /// Parses a raw MOTD from a JSON value (modern chat component).
    pub fn from_json(raw: &Value, bedrock: bool) -> Self {
        let parsed = parse_json_component(raw);
        Self {
            parsed,
            raw: MotdRaw::Json(raw.clone()),
            bedrock,
        }
    }

    /// Simplifies the parsed MOTD in-place.
    pub fn simplify(&mut self) {
        simplify(&mut self.parsed);
    }

    /// Returns the MOTD as plain text (no formatting or color).
    pub fn to_plain(&self) -> String {
        transform::to_plain(&self.parsed)
    }

    /// Returns the MOTD in Minecraft section-sign (§) format.
    pub fn to_minecraft(&self) -> String {
        transform::to_minecraft(&self.parsed)
    }

    /// Returns the MOTD as HTML with inline CSS styles.
    pub fn to_html(&self) -> String {
        transform::to_html(&self.parsed)
    }

    /// Returns the MOTD as ANSI 24-bit color escape codes.
    pub fn to_ansi(&self) -> String {
        transform::to_ansi(&self.parsed)
    }
}

// ── Legacy String Parser ─────────────────────────────────────────────────────

/// Parses a legacy MOTD string with § or & formatting codes.
fn parse_legacy_string(raw: &str) -> ComponentList {
    let mut components = ComponentList::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if (chars[i] == '§' || chars[i] == '&') && i + 1 < chars.len() {
            let code = chars[i + 1];
            // Try color first
            if let Some(color) = MinecraftColor::from_code_char(code) {
                components.push(MotdComponent::MinecraftColor(color));
                i += 2;
                continue;
            }
            // Then formatting
            if let Some(formatting) = Formatting::from_code_char(code) {
                components.push(MotdComponent::Formatting(formatting));
                i += 2;
                continue;
            }
            // Check for hex web color: format is §#RRGGBB or &#RRGGBB
            if code == '#' && i + 7 < chars.len() {
                let hex_str: String = chars[i + 2..i + 8].iter().collect();
                if hex_str.len() == 6
                    && hex_str.chars().all(|c| c.is_ascii_hexdigit())
                {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex_str[0..2], 16),
                        u8::from_str_radix(&hex_str[2..4], 16),
                        u8::from_str_radix(&hex_str[4..6], 16),
                    ) {
                        components.push(MotdComponent::WebColor(WebColor::new(r, g, b)));
                        i += 8;
                        continue;
                    }
                }
            }
            // Not a recognized code — treat as literal text
            components.push(MotdComponent::Text(chars[i].to_string()));
            i += 1;
            continue;
        }

        // Plain text — collect consecutive non-code characters
        let start = i;
        while i < chars.len() {
            if (chars[i] == '§' || chars[i] == '&') && i + 1 < chars.len() {
                let next = chars[i + 1];
                if MinecraftColor::from_code_char(next).is_some()
                    || Formatting::from_code_char(next).is_some()
                    || (next == '#' && i + 7 < chars.len())
                {
                    break;
                }
            }
            i += 1;
        }
        if i > start {
            let text: String = chars[start..i].iter().collect();
            if !text.is_empty() {
                components.push(MotdComponent::Text(text));
            }
        }
    }

    components
}

// ── JSON Chat Component Parser ───────────────────────────────────────────────

/// Parses a modern JSON chat component into a component list.
///
/// Handles the recursive `extra` field and the top-level formatting inheritance.
fn parse_json_component(value: &Value) -> ComponentList {
    let mut components = ComponentList::new();
    parse_json_component_inner(value, &mut components, &Default::default());
    components
}

#[derive(Debug, Clone, Default)]
struct StyleState {
    bold: Option<bool>,
    italic: Option<bool>,
    underlined: Option<bool>,
    strikethrough: Option<bool>,
    obfuscated: Option<bool>,
    color: Option<String>,
}

fn parse_json_component_inner(value: &Value, components: &mut ComponentList, inherited: &StyleState) {
    match value {
        Value::String(s) => {
            // A plain string component
            if !s.is_empty() {
                // Check if this string contains legacy formatting codes
                if s.contains('§') || s.contains('&') {
                    components.extend(parse_legacy_string(s));
                } else {
                    components.push(MotdComponent::Text(s.clone()));
                }
            }
        }
        Value::Object(obj) => {
            // Build the effective style for this component
            let mut style = inherited.clone();

            // Apply color
            if let Some(color_val) = obj.get("color") {
                if let Some(color_str) = color_val.as_str() {
                    style.color = Some(color_str.to_string());
                }
            }

            // Apply formatting
            for (key, target) in [
                ("bold", &mut style.bold),
                ("italic", &mut style.italic),
                ("underlined", &mut style.underlined),
                ("strikethrough", &mut style.strikethrough),
                ("obfuscated", &mut style.obfuscated),
            ] {
                if let Some(val) = obj.get(key) {
                    if let Some(b) = val.as_bool() {
                        *target = Some(b);
                    }
                }
            }

            // Emit color
            if let Some(ref color_str) = style.color {
                if let Some(mc_color) = MinecraftColor::from_code_str(color_str) {
                    components.push(MotdComponent::MinecraftColor(mc_color));
                } else if let Some(web_color) = parse_web_color(color_str) {
                    components.push(MotdComponent::WebColor(web_color));
                }
            }

            // Emit active formatting (colors reset formatting in Minecraft, so emit after color)
            if style.bold == Some(true) {
                components.push(MotdComponent::Formatting(Formatting::Bold));
            }
            if style.italic == Some(true) {
                components.push(MotdComponent::Formatting(Formatting::Italic));
            }
            if style.underlined == Some(true) {
                components.push(MotdComponent::Formatting(Formatting::Underlined));
            }
            if style.strikethrough == Some(true) {
                components.push(MotdComponent::Formatting(Formatting::Strikethrough));
            }
            if style.obfuscated == Some(true) {
                components.push(MotdComponent::Formatting(Formatting::Obfuscated));
            }

            // Translation tag
            if let Some(translate) = obj.get("translate").and_then(|v| v.as_str()) {
                components.push(MotdComponent::TranslationTag {
                    id: translate.to_string(),
                });
            }

            // Text content
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    if text.contains('§') || text.contains('&') {
                        components.extend(parse_legacy_string(text));
                    } else {
                        components.push(MotdComponent::Text(text.to_string()));
                    }
                }
            }

            // Recursively process extra components
            if let Some(extra) = obj.get("extra") {
                if let Some(extra_array) = extra.as_array() {
                    for extra_item in extra_array {
                        parse_json_component_inner(extra_item, components, &style);
                    }
                }
            }
        }
        _ => {
            // Invalid type for a component — ignore
        }
    }
}

impl MinecraftColor {
    /// Parses a Minecraft color from its name string (e.g. "red", "dark_blue").
    fn from_code_str(s: &str) -> Option<Self> {
        match s {
            "black" => Some(MinecraftColor::Black),
            "dark_blue" => Some(MinecraftColor::DarkBlue),
            "dark_green" => Some(MinecraftColor::DarkGreen),
            "dark_aqua" => Some(MinecraftColor::DarkAqua),
            "dark_red" => Some(MinecraftColor::DarkRed),
            "dark_purple" => Some(MinecraftColor::DarkPurple),
            "gold" => Some(MinecraftColor::Gold),
            "gray" => Some(MinecraftColor::Gray),
            "dark_gray" => Some(MinecraftColor::DarkGray),
            "blue" => Some(MinecraftColor::Blue),
            "green" => Some(MinecraftColor::Green),
            "aqua" => Some(MinecraftColor::Aqua),
            "red" => Some(MinecraftColor::Red),
            "light_purple" => Some(MinecraftColor::LightPurple),
            "yellow" => Some(MinecraftColor::Yellow),
            "white" => Some(MinecraftColor::White),
            "minecoin_gold" => Some(MinecraftColor::MinecoinGold),
            _ => None,
        }
    }
}

fn parse_web_color(s: &str) -> Option<WebColor> {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(WebColor::new(r, g, b))
    } else {
        None
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_legacy_simple() {
        let motd = Motd::from_string("§cHello §lWorld", false);
        assert_eq!(motd.to_plain(), "Hello World");
        assert!(motd.to_minecraft().contains("§c"));
        assert!(motd.to_minecraft().contains("§l"));
    }

    #[test]
    fn test_parse_legacy_empty() {
        let motd = Motd::from_string("", false);
        assert!(motd.parsed.is_empty());
    }

    #[test]
    fn test_parse_legacy_no_codes() {
        let motd = Motd::from_string("Just plain text", false);
        assert_eq!(motd.to_plain(), "Just plain text");
        assert_eq!(motd.parsed.len(), 1);
        assert_eq!(motd.parsed[0], MotdComponent::Text("Just plain text".into()));
    }

    #[test]
    fn test_parse_legacy_ampersand_codes() {
        let motd = Motd::from_string("&4Red &lBold", false);
        assert_eq!(motd.to_plain(), "Red Bold");
    }

    #[test]
    fn test_parse_json_simple() {
        let json = serde_json::json!({
            "text": "Hello World",
            "color": "gold"
        });
        let motd = Motd::from_json(&json, false);
        assert_eq!(motd.to_plain(), "Hello World");
        assert!(motd.to_html().contains("#FFAA00"));
    }

    #[test]
    fn test_parse_json_with_extra() {
        let json = serde_json::json!({
            "text": "Prefix ",
            "color": "white",
            "extra": [
                {"text": "Highlight", "color": "red", "bold": true},
                {"text": " Suffix"}
            ]
        });
        let motd = Motd::from_json(&json, false);
        let plain = motd.to_plain();
        assert!(plain.contains("Prefix"));
        assert!(plain.contains("Highlight"));
        assert!(plain.contains("Suffix"));
    }

    #[test]
    fn test_parse_json_nested_extra() {
        let json = serde_json::json!({
            "text": "",
            "extra": [
                {
                    "text": "A",
                    "extra": [
                        {"text": "B", "extra": [{"text": "C"}]}
                    ]
                }
            ]
        });
        let motd = Motd::from_json(&json, false);
        assert_eq!(motd.to_plain(), "ABC");
    }

    #[test]
    fn test_parse_json_translation() {
        let json = serde_json::json!({
            "translate": "multiplayer.player.joined",
            "color": "yellow"
        });
        let motd = Motd::from_json(&json, false);
        let plain = motd.to_plain();
        assert_eq!(plain, "multiplayer.player.joined");
    }

    #[test]
    fn test_motd_simplify_then_transform() {
        // Doubled colors should be simplified
        let mut motd = Motd::from_string("§c§cHello", false);
        assert!(motd.parsed.len() >= 3); // Red, Red, "Hello"
        motd.simplify();
        assert_eq!(motd.parsed.len(), 2); // Red, "Hello"
    }

    #[test]
    fn test_parse_invalid_json_type() {
        let json = serde_json::json!(42);
        let motd = Motd::from_json(&json, false);
        assert!(motd.parsed.is_empty());
    }
}
