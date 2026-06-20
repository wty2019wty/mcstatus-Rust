//! MOTD component types: colors, formatting, and the parsed component enum.

/// A parsed MOTD component — one element in the parsed component list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotdComponent {
    /// Plain text.
    Text(String),
    /// A formatting code (bold, italic, etc.).
    Formatting(Formatting),
    /// A standard Minecraft color (16 named colors).
    MinecraftColor(MinecraftColor),
    /// A hex RGB web color.
    WebColor(WebColor),
    /// A translation tag (e.g. `multiplayer.player.joined`).
    TranslationTag { id: String },
}

impl std::fmt::Display for MotdComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MotdComponent::Text(s) => write!(f, "{s}"),
            MotdComponent::Formatting(fmt) => write!(f, "{fmt:?}"),
            MotdComponent::MinecraftColor(c) => write!(f, "{c:?}"),
            MotdComponent::WebColor(c) => write!(f, "#{:06X}", c.rgb()),
            MotdComponent::TranslationTag { id } => write!(f, "{{{id}}}"),
        }
    }
}

// ── Formatting ───────────────────────────────────────────────────────────────

/// Minecraft text formatting codes.
///
/// These are applied as bitflags in the protocol, but parsed into individual
/// components for ease of manipulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Formatting {
    Bold,
    Italic,
    Underlined,
    Strikethrough,
    Obfuscated,
    /// A reset code (§r) that clears all formatting and color.
    Reset,
}

impl Formatting {
    /// Returns the legacy section-sign code character for this formatting.
    pub fn code_char(&self) -> char {
        match self {
            Formatting::Bold => 'l',
            Formatting::Italic => 'o',
            Formatting::Underlined => 'n',
            Formatting::Strikethrough => 'm',
            Formatting::Obfuscated => 'k',
            Formatting::Reset => 'r',
        }
    }

    /// Parses a formatting code from its section-sign character.
    pub fn from_code_char(c: char) -> Option<Self> {
        match c {
            'l' | 'L' => Some(Formatting::Bold),
            'o' | 'O' => Some(Formatting::Italic),
            'n' | 'N' => Some(Formatting::Underlined),
            'm' | 'M' => Some(Formatting::Strikethrough),
            'k' | 'K' => Some(Formatting::Obfuscated),
            'r' | 'R' => Some(Formatting::Reset),
            _ => None,
        }
    }
}

// ── MinecraftColor ───────────────────────────────────────────────────────────

/// The 16 standard Minecraft named colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MinecraftColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    // Bedrock-only material colors
    MinecoinGold,
}

impl MinecraftColor {
    /// Returns the legacy section-sign code character for this color.
    pub fn code_char(&self) -> char {
        match self {
            MinecraftColor::Black => '0',
            MinecraftColor::DarkBlue => '1',
            MinecraftColor::DarkGreen => '2',
            MinecraftColor::DarkAqua => '3',
            MinecraftColor::DarkRed => '4',
            MinecraftColor::DarkPurple => '5',
            MinecraftColor::Gold => '6',
            MinecraftColor::Gray => '7',
            MinecraftColor::DarkGray => '8',
            MinecraftColor::Blue => '9',
            MinecraftColor::Green => 'a',
            MinecraftColor::Aqua => 'b',
            MinecraftColor::Red => 'c',
            MinecraftColor::LightPurple => 'd',
            MinecraftColor::Yellow => 'e',
            MinecraftColor::White => 'f',
            MinecraftColor::MinecoinGold => 'g',
        }
    }

    /// Returns the RGB hex value for this color.
    pub fn rgb(&self) -> u32 {
        match self {
            MinecraftColor::Black => 0x000000,
            MinecraftColor::DarkBlue => 0x0000AA,
            MinecraftColor::DarkGreen => 0x00AA00,
            MinecraftColor::DarkAqua => 0x00AAAA,
            MinecraftColor::DarkRed => 0xAA0000,
            MinecraftColor::DarkPurple => 0xAA00AA,
            MinecraftColor::Gold => 0xFFAA00,
            MinecraftColor::Gray => 0xAAAAAA,
            MinecraftColor::DarkGray => 0x555555,
            MinecraftColor::Blue => 0x5555FF,
            MinecraftColor::Green => 0x55FF55,
            MinecraftColor::Aqua => 0x55FFFF,
            MinecraftColor::Red => 0xFF5555,
            MinecraftColor::LightPurple => 0xFF55FF,
            MinecraftColor::Yellow => 0xFFFF55,
            MinecraftColor::White => 0xFFFFFF,
            MinecraftColor::MinecoinGold => 0xDDD605,
        }
    }

    /// Returns the red component (0-255).
    pub fn r(&self) -> u8 {
        ((self.rgb() >> 16) & 0xFF) as u8
    }

    /// Returns the green component (0-255).
    pub fn g(&self) -> u8 {
        ((self.rgb() >> 8) & 0xFF) as u8
    }

    /// Returns the blue component (0-255).
    pub fn b(&self) -> u8 {
        (self.rgb() & 0xFF) as u8
    }
        match self {
            MinecraftColor::Black => 0x000000,
            MinecraftColor::DarkBlue => 0x0000AA,
            MinecraftColor::DarkGreen => 0x00AA00,
            MinecraftColor::DarkAqua => 0x00AAAA,
            MinecraftColor::DarkRed => 0xAA0000,
            MinecraftColor::DarkPurple => 0xAA00AA,
            MinecraftColor::Gold => 0xFFAA00,
            MinecraftColor::Gray => 0xAAAAAA,
            MinecraftColor::DarkGray => 0x555555,
            MinecraftColor::Blue => 0x5555FF,
            MinecraftColor::Green => 0x55FF55,
            MinecraftColor::Aqua => 0x55FFFF,
            MinecraftColor::Red => 0xFF5555,
            MinecraftColor::LightPurple => 0xFF55FF,
            MinecraftColor::Yellow => 0xFFFF55,
            MinecraftColor::White => 0xFFFFFF,
            MinecraftColor::MinecoinGold => 0xDDD605,
        }
    }

    /// Parses a color from its section-sign code character.
    pub fn from_code_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            '0' => Some(MinecraftColor::Black),
            '1' => Some(MinecraftColor::DarkBlue),
            '2' => Some(MinecraftColor::DarkGreen),
            '3' => Some(MinecraftColor::DarkAqua),
            '4' => Some(MinecraftColor::DarkRed),
            '5' => Some(MinecraftColor::DarkPurple),
            '6' => Some(MinecraftColor::Gold),
            '7' => Some(MinecraftColor::Gray),
            '8' => Some(MinecraftColor::DarkGray),
            '9' => Some(MinecraftColor::Blue),
            'a' => Some(MinecraftColor::Green),
            'b' => Some(MinecraftColor::Aqua),
            'c' => Some(MinecraftColor::Red),
            'd' => Some(MinecraftColor::LightPurple),
            'e' => Some(MinecraftColor::Yellow),
            'f' => Some(MinecraftColor::White),
            'g' => Some(MinecraftColor::MinecoinGold),
            _ => None,
        }
    }
}

// ── WebColor ─────────────────────────────────────────────────────────────────

/// A hex RGB color used in modern Minecraft chat components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WebColor {
    r: u8,
    g: u8,
    b: u8,
}

impl WebColor {
    /// Creates a new web color from RGB components.
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Returns the packed 24-bit RGB value.
    pub fn rgb(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Returns the red component.
    pub fn r(&self) -> u8 {
        self.r
    }

    /// Returns the green component.
    pub fn g(&self) -> u8 {
        self.g
    }

    /// Returns the blue component.
    pub fn b(&self) -> u8 {
        self.b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_code_chars() {
        assert_eq!(MinecraftColor::from_code_char('f'), Some(MinecraftColor::White));
        assert_eq!(MinecraftColor::from_code_char('F'), Some(MinecraftColor::White));
        assert_eq!(MinecraftColor::from_code_char('0'), Some(MinecraftColor::Black));
        assert_eq!(MinecraftColor::from_code_char('z'), None);
    }

    #[test]
    fn test_formatting_code_chars() {
        assert_eq!(Formatting::from_code_char('l'), Some(Formatting::Bold));
        assert_eq!(Formatting::from_code_char('r'), Some(Formatting::Reset));
        assert_eq!(Formatting::from_code_char('x'), None);
    }

    #[test]
    fn test_color_rgb_values() {
        assert_eq!(MinecraftColor::White.rgb(), 0xFFFFFF);
        assert_eq!(MinecraftColor::Black.rgb(), 0x000000);
        assert_eq!(MinecraftColor::Red.rgb(), 0xFF5555);
    }
}
