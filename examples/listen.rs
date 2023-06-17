use std::path::PathBuf;
use futures::{pin_mut, StreamExt};
use glob::GlobError;
use evdev_shortcut::{Key, Modifier, Shortcut, ShortcutListener};

#[tokio::main]
async fn main() {
    let listener = ShortcutListener::new();
    listener.add(Shortcut::new(&[Modifier::Meta], Key::KeyN));

    let devices =
        glob::glob("/dev/input/by-id/*-kbd").unwrap().collect::<Result<Vec<PathBuf>, GlobError>>().unwrap();

    let stream = listener.listen(&devices).unwrap();

    pin_mut!(stream);

    while let Some(shortcut) = stream.next().await {
        dbg!(shortcut);
    }
}