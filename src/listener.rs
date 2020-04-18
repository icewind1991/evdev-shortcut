use evdev::Device;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::{Shortcut, DeviceOpenError, Key};
use std::path::Path;

pub struct ShortcutListener {
    shortcuts: Arc<Mutex<HashSet<Shortcut>>>,
}

impl ShortcutListener {
    pub fn new() -> Self {
        ShortcutListener {
            shortcuts: Arc::default(),
        }
    }

    pub fn listen<P: AsRef<Path>>(&self, devices: &[P]) -> Result<Receiver<Shortcut>, DeviceOpenError> {
        let mut devices = devices
            .iter()
            .map(|path| Ok(Device::open(path).map_err(|_| DeviceOpenError)?))
            .collect::<Result<Vec<Device>, DeviceOpenError>>()?;

        let (tx, rx) = channel();

        let shortcuts = self.shortcuts.clone();

        std::thread::spawn(move || {
            let mut active_keys = HashSet::new();

            loop {
                let mut got_event = false;

                let events = devices
                    .iter_mut()
                    .flat_map(|device| device.events().unwrap());

                for ev in events {
                    got_event = true;

                    if let Ok(key) = Key::try_from(ev.code) {
                        match ev.value {
                            1 => active_keys.insert(key),
                            0 => active_keys.remove(&key),
                            _ => false,
                        };
                    }
                }

                if got_event {
                    for shortcut in shortcuts.lock().unwrap().iter() {
                        if shortcut.is_triggered(&active_keys) {
                            tx.send(shortcut.clone()).unwrap()
                        }
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        });

        Ok(rx)
    }

    pub fn add(&self, shortcut: Shortcut) {
        self.shortcuts.lock().unwrap().insert(shortcut);
    }

    pub fn remove(&self, shortcut: Shortcut) {
        self.shortcuts.lock().unwrap().remove(&shortcut);
    }
}
