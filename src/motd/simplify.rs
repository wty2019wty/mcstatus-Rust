//! MOTD simplification passes.
//!
//! These functions remove redundant or meaningless formatting/color components
//! from a parsed MOTD component list. The simplifications run iteratively until
//! no more elements can be removed (a fixed-point iteration).

use super::components::{Formatting, MinecraftColor, MotdComponent, WebColor};

/// Type alias for a parsed MOTD component list.
pub type ComponentList = Vec<MotdComponent>;

/// Runs all simplification passes on the component list.
///
/// This runs the 5 removal passes iteratively until no more elements
/// are removed, then applies `squash_nearby_strings`.
pub fn simplify(components: &mut ComponentList) {
    loop {
        let before = components.len();
        remove_double_items(components);
        remove_double_colors(components);
        remove_formatting_before_color(components);
        remove_meaningless_resets_and_colors(components);
        remove_end_non_text(components);
        if components.len() == before {
            break;
        }
    }
    squash_nearby_strings(components);
}

/// Removes duplicate adjacent formatting or color components.
///
/// Two identical colors or formatting codes in a row are redundant —
/// the second one has no effect.
pub fn remove_double_items(components: &mut ComponentList) {
    let mut i = 1;
    while i < components.len() {
        let remove = match (&components[i - 1], &components[i]) {
            (MotdComponent::Formatting(a), MotdComponent::Formatting(b)) => a == b,
            (MotdComponent::MinecraftColor(a), MotdComponent::MinecraftColor(b)) => a == b,
            (MotdComponent::WebColor(a), MotdComponent::WebColor(b)) => a == b,
            _ => false,
        };
        if remove {
            components.remove(i);
        } else {
            i += 1;
        }
    }
}

/// Removes colors that are immediately overridden by another color before any text.
///
/// If a color is followed (with only formatting in between) by another color
/// and no text appears between them, the first color is never seen and can be removed.
pub fn remove_double_colors(components: &mut ComponentList) {
    let mut i = 0;
    while i < components.len() {
        // Find a color at position i
        if !matches!(
            components[i],
            MotdComponent::MinecraftColor(_) | MotdComponent::WebColor(_)
        ) {
            i += 1;
            continue;
        }

        // Look ahead: if we find another color before any text, remove color at i
        let mut j = i + 1;
        let mut only_formatting = true;
        while j < components.len() {
            match &components[j] {
                MotdComponent::Text(_) => {
                    // Text found — the color at i IS visible, keep it
                    only_formatting = false;
                    break;
                }
                MotdComponent::MinecraftColor(_) | MotdComponent::WebColor(_) => {
                    // Another color found before text — remove color at i
                    break;
                }
                MotdComponent::Formatting(_) => {
                    // Formatting is invisible, continue looking ahead
                    j += 1;
                }
                MotdComponent::TranslationTag { .. } => {
                    only_formatting = false;
                    break;
                }
            }
        }

        if only_formatting && j < components.len() {
            components.remove(i);
            // Don't increment i — the next element is now at position i
        } else {
            i += 1;
        }
    }
}

/// Removes formatting codes that appear before a color code.
///
/// In Minecraft, a color code resets all formatting, so any formatting
/// that appears immediately before a color (with no text in between) is
/// never visible and can be removed.
pub fn remove_formatting_before_color(components: &mut ComponentList) {
    let mut i = 0;
    while i < components.len() {
        if !matches!(components[i], MotdComponent::Formatting(_)) {
            i += 1;
            continue;
        }

        // Look ahead for a color before any text
        let mut j = i + 1;
        let mut found_color = false;
        while j < components.len() {
            match &components[j] {
                MotdComponent::Text(_) | MotdComponent::TranslationTag { .. } => break,
                MotdComponent::MinecraftColor(_) | MotdComponent::WebColor(_) => {
                    found_color = true;
                    break;
                }
                MotdComponent::Formatting(_) => {
                    j += 1;
                }
            }
        }

        if found_color {
            components.remove(i);
        } else {
            i += 1;
        }
    }
}

/// Removes unnecessary reset codes and redundant color/formatting sets.
///
/// - A reset followed by the same formatting/color that was already active is redundant
/// - A reset at the very beginning of the component list has no effect
/// - A color re-setting the same color is redundant
pub fn remove_meaningless_resets_and_colors(components: &mut ComponentList) {
    // Remove leading resets
    while components
        .first()
        .map_or(false, |c| matches!(c, MotdComponent::Formatting(Formatting::Reset)))
    {
        components.remove(0);
    }

    // Remove reset followed by re-application of the same color
    let mut i = 1;
    while i < components.len() {
        if matches!(components[i - 1], MotdComponent::Formatting(Formatting::Reset)) {
            // Track what was active before the reset
            let mut active_color: Option<ColorState> = None;
            let mut active_formatting: Vec<Formatting> = Vec::new();

            // Walk backward from i-2 to find the active state
            for k in (0..i.saturating_sub(1)).rev() {
                match &components[k] {
                    MotdComponent::MinecraftColor(c) => {
                        active_color = Some(ColorState::Minecraft(*c));
                        break;
                    }
                    MotdComponent::WebColor(c) => {
                        active_color = Some(ColorState::Web(*c));
                        break;
                    }
                    MotdComponent::Formatting(f) if *f != Formatting::Reset => {
                        if !active_formatting.contains(f) {
                            active_formatting.push(*f);
                        }
                    }
                    MotdComponent::Formatting(Formatting::Reset) => {
                        // Previous reset clears state
                        active_color = None;
                        active_formatting.clear();
                    }
                    _ => {}
                }
            }

            // Check if the element after reset re-applies the same color
            if i < components.len() {
                let re_applies_same = match (&components[i], &active_color) {
                    (MotdComponent::MinecraftColor(c), Some(ColorState::Minecraft(ac))) => c == ac,
                    (MotdComponent::WebColor(c), Some(ColorState::Web(ac))) => c == ac,
                    _ => false,
                };
                if re_applies_same {
                    components.remove(i); // Remove the re-applied color
                    components.remove(i - 1); // Remove the reset
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Removes trailing formatting and color codes that have no text after them.
pub fn remove_end_non_text(components: &mut ComponentList) {
    while components
        .last()
        .map_or(false, |c| !matches!(c, MotdComponent::Text(_) | MotdComponent::TranslationTag { .. }))
    {
        components.pop();
    }
}

/// Merges adjacent plain text strings into a single component.
pub fn squash_nearby_strings(components: &mut ComponentList) {
    let mut i = 1;
    while i < components.len() {
        if let (MotdComponent::Text(a), MotdComponent::Text(b)) = (&components[i - 1], &components[i]) {
            let merged = format!("{a}{b}");
            components[i - 1] = MotdComponent::Text(merged);
            components.remove(i);
        } else {
            i += 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ColorState {
    Minecraft(MinecraftColor),
    Web(WebColor),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_double_items() {
        let mut components = vec![
            MotdComponent::MinecraftColor(MinecraftColor::Red),
            MotdComponent::MinecraftColor(MinecraftColor::Red),
            MotdComponent::Text("Hello".into()),
        ];
        remove_double_items(&mut components);
        // The second Red should be removed
        assert_eq!(components.len(), 2);
        assert_eq!(components[1], MotdComponent::Text("Hello".into()));
    }

    #[test]
    fn test_remove_end_non_text() {
        let mut components = vec![
            MotdComponent::Text("Hi".into()),
            MotdComponent::MinecraftColor(MinecraftColor::Red),
            MotdComponent::Formatting(Formatting::Bold),
        ];
        remove_end_non_text(&mut components);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], MotdComponent::Text("Hi".into()));
    }

    #[test]
    fn test_squash_nearby_strings() {
        let mut components = vec![
            MotdComponent::Text("Hello".into()),
            MotdComponent::Text(" ".into()),
            MotdComponent::Text("World".into()),
        ];
        squash_nearby_strings(&mut components);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], MotdComponent::Text("Hello World".into()));
    }

    #[test]
    fn test_remove_leading_reset() {
        let mut components = vec![
            MotdComponent::Formatting(Formatting::Reset),
            MotdComponent::Text("Hi".into()),
        ];
        remove_meaningless_resets_and_colors(&mut components);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], MotdComponent::Text("Hi".into()));
    }

    #[test]
    fn test_simplify_no_changes() {
        let original = vec![
            MotdComponent::Text("Simple MOTD".into()),
        ];
        let mut simplified = original.clone();
        simplify(&mut simplified);
        assert_eq!(simplified, original);
    }
}
