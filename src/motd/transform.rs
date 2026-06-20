//! MOTD output transformers.
//!
//! Transforms a parsed MOTD component list into various output formats:
//! plain text, Minecraft section-sign format, HTML, and ANSI 24-bit color.

use super::components::{Formatting, MotdComponent};
use super::simplify::ComponentList;

/// Transforms a component list into plain text, stripping all formatting and color.
pub fn to_plain(components: &ComponentList) -> String {
    let mut result = String::new();
    for component in components {
        match component {
            MotdComponent::Text(s) => result.push_str(s),
            MotdComponent::TranslationTag { id } => result.push_str(id),
            _ => {}
        }
    }
    result
}

/// Transforms a component list into Minecraft section-sign (§) format.
pub fn to_minecraft(components: &ComponentList) -> String {
    let mut result = String::new();
    for component in components {
        match component {
            MotdComponent::Text(s) => result.push_str(s),
            MotdComponent::Formatting(f) => {
                result.push('§');
                result.push(f.code_char());
            }
            MotdComponent::MinecraftColor(c) => {
                result.push('§');
                result.push(c.code_char());
            }
            MotdComponent::WebColor(c) => {
                // Web colors are rendered as their hex value in section-sign format
                result.push_str(&format!("§#{}", hex::encode_upper(&[c.r(), c.g(), c.b()])));
            }
            MotdComponent::TranslationTag { id } => {
                result.push_str(id);
            }
        }
    }
    result
}

/// Transforms a component list into HTML with inline CSS styles.
pub fn to_html(components: &ComponentList) -> String {
    let mut result = String::new();
    let mut open_tags: Vec<String> = Vec::new();

    for component in components {
        match component {
            MotdComponent::Text(s) => result.push_str(&html_escape(s)),
            MotdComponent::Formatting(f) => match f {
                Formatting::Reset => {
                    // Close all open tags
                    close_all_tags(&mut result, &mut open_tags);
                }
                Formatting::Bold => {
                    result.push_str("<span style=\"font-weight: bold\">");
                    open_tags.push("</span>".into());
                }
                Formatting::Italic => {
                    result.push_str("<span style=\"font-style: italic\">");
                    open_tags.push("</span>".into());
                }
                Formatting::Underlined => {
                    result.push_str("<span style=\"text-decoration: underline\">");
                    open_tags.push("</span>".into());
                }
                Formatting::Strikethrough => {
                    result.push_str("<span style=\"text-decoration: line-through\">");
                    open_tags.push("</span>".into());
                }
                Formatting::Obfuscated => {
                    result.push_str("<span class=\"obfuscated\">");
                    open_tags.push("</span>".into());
                }
            },
            MotdComponent::MinecraftColor(c) => {
                // Colors reset formatting in Minecraft
                close_all_tags(&mut result, &mut open_tags);
                let color = format!("#{:06X}", c.rgb());
                result.push_str(&format!("<span style=\"color: {color}\">"));
                open_tags.push("</span>".into());
            }
            MotdComponent::WebColor(c) => {
                close_all_tags(&mut result, &mut open_tags);
                let color = format!("#{:06X}", c.rgb());
                result.push_str(&format!("<span style=\"color: {color}\">"));
                open_tags.push("</span>".into());
            }
            MotdComponent::TranslationTag { id } => {
                result.push_str(&html_escape(id));
            }
        }
    }

    close_all_tags(&mut result, &mut open_tags);
    result
}

/// Transforms a component list into ANSI 24-bit color escape codes.
pub fn to_ansi(components: &ComponentList) -> String {
    let mut result = String::new();
    let mut current_fg: Option<u32> = None;

    for component in components {
        match component {
            MotdComponent::Text(s) => result.push_str(s),
            MotdComponent::Formatting(f) => match f {
                Formatting::Reset => {
                    result.push_str("\x1b[0m");
                    current_fg = None;
                }
                Formatting::Bold => result.push_str("\x1b[1m"),
                Formatting::Italic => result.push_str("\x1b[3m"),
                Formatting::Underlined => result.push_str("\x1b[4m"),
                Formatting::Strikethrough => result.push_str("\x1b[9m"),
                Formatting::Obfuscated => {} // Cannot represent obfuscated in ANSI
            },
            MotdComponent::MinecraftColor(c) => {
                // Apply foreground color (24-bit if not already set)
                let rgb = c.rgb();
                if current_fg != Some(rgb) {
                    result.push_str(&format!("\x1b[38;2;{};{};{}m", c.r(), c.g(), c.b()));
                    current_fg = Some(rgb);
                }
            }
            MotdComponent::WebColor(c) => {
                let rgb = c.rgb();
                if current_fg != Some(rgb) {
                    result.push_str(&format!("\x1b[38;2;{};{};{}m", c.r(), c.g(), c.b()));
                    current_fg = Some(rgb);
                }
            }
            MotdComponent::TranslationTag { id } => {
                result.push_str(id);
            }
        }
    }

    // Reset at end
    if !result.is_empty() {
        result.push_str("\x1b[0m");
    }
    result
}

fn close_all_tags(result: &mut String, open_tags: &mut Vec<String>) {
    while let Some(tag) = open_tags.pop() {
        result.push_str(&tag);
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Internal hex encoding utility.
mod hex {
    pub fn encode_upper(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motd::components::MinecraftColor;

    #[test]
    fn test_to_plain() {
        let components = vec![
            MotdComponent::MinecraftColor(MinecraftColor::Red),
            MotdComponent::Formatting(Formatting::Bold),
            MotdComponent::Text("Hello".into()),
            MotdComponent::Formatting(Formatting::Reset),
            MotdComponent::Text(" World".into()),
        ];
        assert_eq!(to_plain(&components), "Hello World");
    }

    #[test]
    fn test_to_minecraft() {
        let components = vec![
            MotdComponent::MinecraftColor(MinecraftColor::Red),
            MotdComponent::Text("Red text".into()),
        ];
        assert_eq!(to_minecraft(&components), "§cRed text");
    }

    #[test]
    fn test_to_html() {
        let components = vec![
            MotdComponent::MinecraftColor(MinecraftColor::Gold),
            MotdComponent::Text("Gold text".into()),
        ];
        let html = to_html(&components);
        assert!(html.contains("<span style=\"color: #FFAA00\">"));
        assert!(html.contains("Gold text"));
        assert!(html.contains("</span>"));
    }

    #[test]
    fn test_to_ansi() {
        let components = vec![
            MotdComponent::MinecraftColor(MinecraftColor::Red),
            MotdComponent::Text("Red".into()),
            MotdComponent::Formatting(Formatting::Reset),
        ];
        let ansi = to_ansi(&components);
        assert!(ansi.contains("\x1b[38;2;255;85;85m")); // Red RGB
        assert!(ansi.contains("Red"));
        assert!(ansi.contains("\x1b[0m"));
    }
}
