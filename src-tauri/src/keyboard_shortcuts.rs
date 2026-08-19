use std::str::FromStr;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

fn normalize_aliases(shortcut: &str) -> String {
    shortcut
        .replace("CmdOrCtrl", "Super")
        .replace("Command", "Super")
        .replace("Cmd", "Super")
        .replace("Option", "Alt")
        .replace("Control", "Ctrl")
}

pub fn parse(shortcut: &str) -> Option<Shortcut> {
    let shortcut = shortcut.trim();
    if shortcut.is_empty() {
        return None;
    }
    if let Ok(parsed) = Shortcut::from_str(shortcut) {
        return Some(parsed);
    }
    let normalized = normalize_aliases(shortcut);
    if let Ok(parsed) = Shortcut::from_str(&normalized) {
        return Some(parsed);
    }
    let parts: Vec<&str> = normalized.split('+').collect();
    let key = parts.last()?.trim();
    let converted_key = if key.len() == 1 && key.chars().next()?.is_ascii_alphabetic() {
        format!("Key{}", key.to_ascii_uppercase())
    } else if key.len() == 1 && key.chars().next()?.is_ascii_digit() {
        format!("Digit{key}")
    } else {
        return None;
    };
    Shortcut::from_str(&format!(
        "{}+{converted_key}",
        parts[..parts.len() - 1].join("+")
    ))
    .ok()
}

pub fn parse_for_current_layout(shortcut: &str) -> Option<Vec<Shortcut>> {
    let shortcut = shortcut.trim();
    if shortcut.is_empty() {
        return None;
    }
    let normalized = normalize_aliases(shortcut);
    let mut shortcuts: Vec<Shortcut> = parse(&normalized).into_iter().collect();
    let parts: Vec<&str> = normalized.split('+').collect();
    if let Some(key) = parts.last().filter(|key| key.trim().len() == 1) {
        let character = key.trim().chars().next()?;
        let mut modifiers = Modifiers::empty();
        for modifier in &parts[..parts.len() - 1] {
            match modifier.trim() {
                "Super" => modifiers |= Modifiers::SUPER,
                "Alt" => modifiers |= Modifiers::ALT,
                "Ctrl" => modifiers |= Modifiers::CONTROL,
                "Shift" => modifiers |= Modifiers::SHIFT,
                _ => {}
            }
        }
        let command_modifier = modifiers.intersects(Modifiers::SUPER | Modifiers::META);
        if let Some(code) = crate::keyboard_layout::code_for_character(character, command_modifier)
        {
            shortcuts.clear();
            shortcuts.push(Shortcut::new(Some(modifiers), code));
        }
    }
    (!shortcuts.is_empty()).then_some(shortcuts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_product_aliases_letters_and_digits() {
        for shortcut in [
            "CmdOrCtrl+Shift+V",
            "Command+Option+C",
            "Control+1",
            "Alt+Shift+KeyX",
        ] {
            assert!(parse(shortcut).is_some(), "could not parse {shortcut}");
        }
        assert!(parse("").is_none());
        assert!(parse("definitely not a shortcut").is_none());
    }
}
