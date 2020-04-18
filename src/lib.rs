pub use keycodes::Key;
use parse_display::{Display, FromStr, ParseError};
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::str::FromStr;
use err_derive::Error;

mod keycodes;

#[cfg(feature = "foo")]
mod listener;
#[cfg(feature = "foo")]
pub use listener::ShortcutListener;

#[derive(Debug, Clone, Error)]
#[error(display = "Failed to open device")]
pub struct DeviceOpenError;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Display, FromStr)]
pub enum Modifier {
    Alt,
    LeftAlt,
    RightAlt,
    Ctrl,
    LeftCtrl,
    RightCtrl,
    Shift,
    LeftShift,
    RightShift,
    Meta,
    LeftMeta,
    RightMeta,
}

impl Modifier {
    pub fn as_mask(&self) -> u8 {
        match self {
            Modifier::Alt => 0b00000011,
            Modifier::LeftAlt => 0b00000001,
            Modifier::RightAlt => 0b00000010,
            Modifier::Ctrl => 0b00001100,
            Modifier::LeftCtrl => 0b00000100,
            Modifier::RightCtrl => 0b00001000,
            Modifier::Meta => 0b00110000,
            Modifier::LeftMeta => 0b00010000,
            Modifier::RightMeta => 0b00100000,
            Modifier::Shift => 0b11000000,
            Modifier::LeftShift => 0b01000000,
            Modifier::RightShift => 0b10000000,
        }
    }

    pub fn mask_from_key(key: Key) -> u8 {
        match key {
            Key::KeyLeftAlt => 0b00000001,
            Key::KeyRightAlt => 0b00000010,
            Key::KeyLeftCtrl => 0b00000100,
            Key::KeyRightCtrl => 0b00001000,
            Key::KeyLeftMeta => 0b00010000,
            Key::KeyRightMeta => 0b00100000,
            Key::KeyLeftShift => 0b01000000,
            Key::KeyRightShift => 0b10000000,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ModifierList(Vec<Modifier>);

impl ModifierList {
    pub fn as_mask(&self) -> u8 {
        self.0
            .iter()
            .fold(0, |mask, modifier| mask | modifier.as_mask())
    }
}

impl Display for ModifierList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for modifier in self.0.iter() {
            write!(f, "<{}>", modifier)?;
        }
        Ok(())
    }
}

impl FromStr for ModifierList {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ModifierList(
            s.split('>')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    if !part.starts_with('<') {
                        Err(ParseError::with_message("Invalid modifier"))
                    } else {
                        Ok(part[1..].parse::<Modifier>()?)
                    }
                })
                .collect::<Result<Vec<Modifier>, ParseError>>()?,
        ))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Display, FromStr)]
#[display("{modifiers}-{key}")]
pub struct Shortcut {
    modifiers: ModifierList,
    key: Key,
}

#[cfg(test)]
mod tests {
    use crate::keyboard::{Key, Modifier, Shortcut};
    use test_case::test_case;

    #[test_case("<Ctrl>-KeyP", Shortcut::new(vec ! [Modifier::Ctrl], Key::KeyP))]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", Shortcut::new(vec ! [Modifier::LeftCtrl, Modifier::LeftAlt], Key::KeyLeft))]
    fn shortcut_parse_display_test(s: &str, shortcut: Shortcut) {
        assert_eq!(s, format!("{}", shortcut));

        assert_eq!(shortcut, s.parse().unwrap());
    }
}

impl Shortcut {
    pub fn new(modifiers: Vec<Modifier>, key: Key) -> Self {
        Shortcut {
            modifiers: ModifierList(modifiers),
            key,
        }
    }

    pub fn identifier(&self) -> String {
        self.to_string()
            .replace('<', "")
            .replace('>', "")
            .replace('-', "_")
    }
}

impl Shortcut {
    pub fn is_triggered(&self, active_keys: &HashSet<Key>) -> bool {
        let desired_mask = self.modifiers.as_mask();
        let pressed_mask = active_keys
            .iter()
            .fold(0, |mask, key| mask | Modifier::mask_from_key(*key));

        let desired_presses = desired_mask & pressed_mask;
        let modifiers_match = (desired_presses == pressed_mask)
            && (desired_presses.count_ones() == self.modifiers.0.len() as u32);

        modifiers_match && active_keys.contains(&self.key)
    }
}

#[cfg(test)]
mod triggered_tests {
    use crate::keyboard::{Key, Shortcut};
    use test_case::test_case;

    #[test_case("<Ctrl>-KeyP", & [] => false)]
    #[test_case("<Ctrl>-KeyP", & [Key::KeyLeftCtrl, Key::KeyP] => true)]
    #[test_case("<Ctrl>-KeyP", & [Key::KeyRightCtrl, Key::KeyP] => true)]
    #[test_case("<LeftCtrl>-KeyP", & [Key::KeyLeftCtrl, Key::KeyP] => true)]
    #[test_case("<LeftCtrl>-KeyP", & [Key::KeyRightCtrl, Key::KeyP] => false)]
    #[test_case("<Ctrl>-KeyP", & [Key::KeyLeftCtrl, Key::KeyLeftAlt, Key::KeyP] => false)]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", & [] => false)]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", & [Key::KeyLeft] => false)]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", & [Key::KeyLeftCtrl, Key::KeyLeft] => false)]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", & [Key::KeyLeftCtrl, Key::KeyLeftAlt] => false)]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", & [Key::KeyLeftCtrl, Key::KeyLeftAlt, Key::KeyRight] => false)]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", & [Key::KeyLeftCtrl, Key::KeyLeftAlt, Key::KeyLeft] => true)]
    #[test_case("<LeftCtrl><LeftAlt>-KeyLeft", & [Key::KeyLeftCtrl, Key::KeyRightAlt, Key::KeyLeft] => false)]
    #[test_case("<Ctrl><Alt>-KeyLeft", & [Key::KeyLeftCtrl, Key::KeyRightAlt, Key::KeyLeft] => true)]
    fn shortcut_triggered(s: &str, keys: &[Key]) -> bool {
        let shortcut: Shortcut = s.parse().unwrap();
        shortcut.is_triggered(&keys.into_iter().copied().collect())
    }
}