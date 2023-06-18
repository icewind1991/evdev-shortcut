use evdev::Device;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
use crate::{Shortcut, DeviceOpenError, Key, ShortcutEvent, ShortcutState};
use std::path::Path;
use async_stream::stream;
use futures::pin_mut;
use futures::{Stream, StreamExt};
use futures::stream::{iter};
use tracing::{debug, trace, info};

/// A listener for shortcut events
///
/// Example:
///
/// ```rust,no_run
/// # use std::path::PathBuf;
/// # use glob::GlobError;
/// # use evdev_shortcut::{ShortcutListener, Shortcut, Modifier, Key};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let listener = ShortcutListener::new();
/// listener.add(Shortcut::new(&[Modifier::Meta], Key::KeyN));
///
/// let devices =
///     glob::glob("/dev/input/by-id/*-kbd")?.collect::<Result<Vec<PathBuf>, GlobError>>()?;
///
/// let stream = listener.listen(&devices)?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct ShortcutListener {
    shortcuts: Arc<Mutex<HashSet<Shortcut>>>,
}

impl ShortcutListener {
    pub fn new() -> Self {
        ShortcutListener::default()
    }

    /// Listen for shortcuts on the provided set of input devices.
    ///
    /// Note that you need to register shortcuts using [add](ShortcutListener::add) to get any events.
    pub fn listen<P: AsRef<Path>>(&self, devices: &[P]) -> Result<impl Stream<Item=ShortcutEvent>, DeviceOpenError> {
        let shortcuts = self.shortcuts.clone();

        let devices = devices
            .iter()
            .map(|path| {
                let path = path.as_ref();
                let res = Device::open(path).map_err(|_| DeviceOpenError { device: path.into() });
                debug!(device = ?path, success = res.is_ok(), "opening input device");
                res
            })
            .collect::<Result<Vec<Device>, DeviceOpenError>>()?;
        let events = iter(devices.into_iter().flat_map(|device| device.into_event_stream()))
            .flatten();

        Ok(stream! {
            let mut active_keys = HashSet::new();
            let mut pressed_shortcuts = HashSet::new();

            pin_mut!(events);

            while let Some(Ok(event)) = events.next().await {
                trace!(?event, "evdev event");
                if let Ok(key) = Key::try_from(event.code()) {
                    match event.value() {
                        1 => active_keys.insert(key),
                        0 => active_keys.remove(&key),
                        _ => false,
                    };
                }

                let shortcuts: Vec<_> = shortcuts.lock().unwrap().iter().cloned().collect();

                for shortcut in shortcuts {
                    let is_triggered = shortcut.is_triggered(&active_keys);
                    let was_triggered = pressed_shortcuts.contains(&shortcut);
                    if is_triggered && !was_triggered {
                        pressed_shortcuts.insert(shortcut.clone());
                        info!(?shortcut, "pressed");
                        yield ShortcutEvent {
                            shortcut,
                            state: ShortcutState::Pressed,
                        };
                    } else if !is_triggered && was_triggered {
                        pressed_shortcuts.remove(&shortcut);
                        info!(?shortcut, "released");
                        yield ShortcutEvent {
                            shortcut,
                            state: ShortcutState::Released,
                        };
                    }
                }
            }
        })
    }

    /// Returns `true` if the shortcut was not previously listened to
    pub fn add(&self, shortcut: Shortcut) -> bool {
        self.shortcuts.lock().unwrap().insert(shortcut)
    }

    /// Returns `true` if the shortcut was previously listened to
    pub fn remove(&self, shortcut: &Shortcut) -> bool {
        self.shortcuts.lock().unwrap().remove(shortcut)
    }

    /// Check if a shortcut is currently being listened for
    pub fn has(&self, shortcut: &Shortcut) -> bool {
        self.shortcuts.lock().unwrap().contains(shortcut)
    }
}
