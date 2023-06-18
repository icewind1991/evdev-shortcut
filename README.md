# evdev-shortcut

Global shortcuts using evdev

## Usage

```rust
use std::path::PathBuf;
use glob::GlobError;
use evdev_shortcut::{ShortcutListener, Shortcut, Modifier, Key};
use tokio::pin;
use futures::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = ShortcutListener::new();
    listener.add(Shortcut::new(&[Modifier::Meta], Key::KeyN));
    
    let devices =
        glob::glob("/dev/input/by-id/*-kbd")?.collect::<Result<Vec<PathBuf>, GlobError>>()?;
    
    let stream = listener.listen(&devices)?;
    pin!(stream);
    
    while let Some(event) = stream.next().await {
        println!("{} {}", event.shortcut, event.state);
    }
    Ok(())
}
```

Note that raw access to evdev devices is a privileged operation and usually requires running with elevated privileges.
See [shortcutd](https://github.com/icewind1991/shortcutd) for a solution to running the elevated input handling in a separate process.