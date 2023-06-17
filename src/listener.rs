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

#[derive(Default)]
pub struct ShortcutListener {
    shortcuts: Arc<Mutex<HashSet<Shortcut>>>,
}

impl ShortcutListener {
    pub fn new() -> Self {
        ShortcutListener::default()
    }

    pub fn listen<P: AsRef<Path>>(&self, devices: &[P]) -> Result<impl Stream<Item=ShortcutEvent>, DeviceOpenError> {
        let shortcuts = self.shortcuts.clone();

        let devices = devices
            .iter()
            .map(|path| Device::open(path).map_err(|_| DeviceOpenError))
            .collect::<Result<Vec<Device>, DeviceOpenError>>()?;
        let events = iter(devices.into_iter().flat_map(|device| device.into_event_stream()))
            .flatten();

        Ok(stream! {
            let mut active_keys = HashSet::new();
            let mut pressed_shortcuts = HashSet::new();

            pin_mut!(events);

            while let Some(Ok(event)) = events.next().await {
                // dbg!(&event);
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
                        yield ShortcutEvent {
                            shortcut,
                            state: ShortcutState::Pressed,
                        };
                    } else if !is_triggered && was_triggered {
                        pressed_shortcuts.remove(&shortcut);
                        yield ShortcutEvent {
                            shortcut,
                            state: ShortcutState::Released,
                        };
                    }
                }
            }
        })
    }

    pub fn add(&self, shortcut: Shortcut) {
        self.shortcuts.lock().unwrap().insert(shortcut);
    }

    pub fn remove(&self, shortcut: Shortcut) {
        self.shortcuts.lock().unwrap().remove(&shortcut);
    }
}
