//! Global keyboard shortcuts using evdev.
//!
//! By connecting to the input devices directly with evdev the shortcuts can work regardless of the environment,
//! they will work under X11, wayland and in the terminal.
//!
//! This does come at the cost of having to run the program with elevated permissions.
//! See [shortcutd](https://docs.rs/shortcutd/latest/shortcutd/) for a solution to running the elevated input handling in a separate process.
//!
//! Example:
//!
//! ```rust,no_run
//! # use std::path::PathBuf;
//! # use glob::GlobError;
//! # use evdev_shortcut::{ShortcutListener, Shortcut, Modifier, Key};
//! # use tokio::pin;
//! # use futures::stream::StreamExt;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let listener = ShortcutListener::new();
//! listener.add(Shortcut::new(&[Modifier::Meta], Key::KeyN));
//!
//! let devices =
//!     glob::glob("/dev/input/by-id/*-kbd")?.collect::<Result<Vec<PathBuf>, GlobError>>()?;
//!
//! let stream = listener.listen(&devices)?;
//! pin!(stream);
//!
//! while let Some(event) = stream.next().await {
//!     println!("{} {}", event.shortcut, event.state);
//! }
//! # Ok(())
//! # }
//! ```

pub use keycodes::Key;
use parse_display::{Display, FromStr, ParseError};
use std::collections::HashSet;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

mod keycodes;

#[cfg(feature = "listener")]
mod listener;

#[cfg(feature = "listener")]
pub use listener::ShortcutListener;

/// Error emitted when an input device can't be opened
#[derive(Debug, Clone, Error)]
#[error("Failed to open device {device:?}")]
pub struct DeviceOpenError {
    pub device: PathBuf,
}

/// Modifier key for shortcuts
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Display, FromStr)]
#[repr(u8)]
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

const ALL_MODIFIERS: &[Modifier] = &[
    Modifier::Alt,
    Modifier::LeftAlt,
    Modifier::RightAlt,
    Modifier::Ctrl,
    Modifier::LeftCtrl,
    Modifier::RightCtrl,
    Modifier::Shift,
    Modifier::LeftShift,
    Modifier::RightShift,
    Modifier::Meta,
    Modifier::LeftMeta,
    Modifier::RightMeta,
];

const COMBINED_MODIFIERS: &[Modifier] = &[
    Modifier::Alt,
    Modifier::Ctrl,
    Modifier::Shift,
    Modifier::Meta,
];

impl Modifier {
    pub fn mask(&self) -> u8 {
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

/// Set of modifier keys for shortcuts
#[derive(Clone, Debug, Hash, PartialEq, Eq, Copy, Default)]
pub struct ModifierList(u8);

impl ModifierList {
    pub fn new(modifiers: &[Modifier]) -> Self {
        ModifierList(modifiers
            .iter()
            .fold(0, |mask, modifier| mask | modifier.mask()))
    }

    pub fn mask(&self) -> u8 {
        self.0
    }

    pub fn modifiers(&self) -> impl Iterator<Item=Modifier> {
        let mask = self.mask();
        ALL_MODIFIERS.iter().copied().filter(move |modifier| {
            for combined in COMBINED_MODIFIERS {
                // if <Ctrl> is enabled, don't emit <LeftCtrl> and <RightCtrl>
                if combined != modifier && combined.mask() & modifier.mask() == modifier.mask() && combined.mask() & mask == combined.mask() {
                    return false;
                }
            }
            modifier.mask() & mask == modifier.mask()
        })
    }

    pub fn len(&self) -> u32 {
        self.modifiers().count() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.mask() == 0
    }
}

impl Display for ModifierList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for modifier in self.modifiers() {
            write!(f, "<{}>", modifier)?;
        }
        Ok(())
    }
}

impl FromStr for ModifierList {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let modifiers = s.split('>')
            .filter(|part| !part.is_empty())
            .map(|part| {
                if !part.starts_with('<') {
                    Err(ParseError::with_message("Invalid modifier"))
                } else {
                    Ok(part[1..].parse::<Modifier>()?)
                }
            })
            .collect::<Result<Vec<Modifier>, ParseError>>()?;
        Ok(ModifierList::new(&modifiers))
    }
}

/// A keyboard shortcut consisting of zero or more modifier keys and a non-modifier key
///
/// Examples:
///
/// Create from keys:
///
/// ```rust
/// # use evdev_shortcut::{Shortcut, Modifier, Key};
/// # fn main() {
/// let shortcut = Shortcut::new(&[Modifier::Meta], Key::KeyN);
/// # }
/// ```
///
/// Parse from string:
///
/// ```rust
/// # use evdev_shortcut::{Shortcut, Modifier, Key};
/// # use std::str::FromStr;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let shortcut: Shortcut = "<Meta>-KeyN".parse()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Shortcut {
    pub modifiers: ModifierList,
    pub key: Key,
}

impl FromStr for Shortcut {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((modifiers, key)) = s.split_once('-') {
            Ok(Shortcut {
                modifiers: modifiers.parse()?,
                key: key.parse()?,
            })
        } else {
            Ok(Shortcut {
                modifiers: ModifierList::default(),
                key: s.parse()?,
            })
        }
    }
}

impl Display for Shortcut {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.modifiers.is_empty() {
            write!(f, "{}", self.key)
        } else {
            write!(f, "{}-{}", self.modifiers, self.key)
        }
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;
    use crate::{Key, Modifier, ModifierList, Shortcut};

    #[test_case("KeyP", Shortcut::new(& [], Key::KeyP))]
    #[test_case("<Ctrl>-KeyP", Shortcut::new(& [Modifier::Ctrl], Key::KeyP))]
    #[test_case("<LeftAlt><LeftCtrl>-KeyLeft", Shortcut::new(& [Modifier::LeftCtrl, Modifier::LeftAlt], Key::KeyLeft))]
    fn shortcut_parse_display_test(s: &str, shortcut: Shortcut) {
        assert_eq!(s, format!("{}", shortcut));

        assert_eq!(shortcut, s.parse().unwrap());
    }

    #[test_case(& [Modifier::Ctrl])]
    #[test_case(& [Modifier::LeftAlt, Modifier::LeftCtrl])]
    #[test_case(& [Modifier::Shift, Modifier::Meta])]
    fn test_modifier_list(modifiers: &[Modifier]) {
        assert_eq!(modifiers.to_vec(), ModifierList::new(modifiers).modifiers().collect::<Vec<_>>())
    }
}

impl Shortcut {
    pub fn new(modifiers: &[Modifier], key: Key) -> Self {
        Shortcut {
            modifiers: ModifierList::new(modifiers),
            key,
        }
    }

    pub fn identifier(&self) -> String {
        self.to_string()
            .replace(['<', '>'], "")
            .replace('-', "_")
    }
}

impl Shortcut {
    pub fn is_triggered(&self, active_keys: &HashSet<Key>) -> bool {
        let desired_mask = self.modifiers.mask();
        let pressed_mask = active_keys
            .iter()
            .fold(0, |mask, key| mask | Modifier::mask_from_key(*key));

        let desired_presses = desired_mask & pressed_mask;
        let modifiers_match = (desired_presses == pressed_mask)
            && (desired_presses.count_ones() == self.modifiers.len());

        modifiers_match && active_keys.contains(&self.key)
    }
}

#[cfg(test)]
mod triggered_tests {
    use crate::{Key, Shortcut};
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

/// Whether the shortcut was pressed or released
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ShortcutState {
    Pressed,
    Released,
}

impl ShortcutState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShortcutState::Pressed => "pressed",
            ShortcutState::Released => "released",
        }
    }
}

impl Display for ShortcutState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Event emitted when a shortcut is pressed or released.
#[derive(Debug, Clone)]
pub struct ShortcutEvent {
    pub shortcut: Shortcut,
    pub state: ShortcutState,
}