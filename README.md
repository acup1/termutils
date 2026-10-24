# termutils

A small Rust library for terminal raw mode, alternate-screen rendering, mouse
input, drawing primitives, and layered virtual windows.

```rust,no_run
use std::io;
use termutils::{terminal::Terminal, WindowManager};

fn main() -> io::Result<()> {
    let mut terminal = Terminal::wannajustwork()?;
    let mut windows = WindowManager::new();
    let id = windows.new_window(0);

    let window = windows.get_mut(id).unwrap();
    window.x = 2;
    window.y = 1;
    window.width = 20;
    window.height = 5;
    window.name = Some("termutils".into());
    window.posprint(1, 1, "press q to quit");

    loop {
        windows.render_windows(&mut terminal)?;
        if terminal.getch(20) == 'q' {
            break;
        }
    }
    Ok(())
}
```

`Terminal` restores raw mode, mouse tracking, the cursor, and the alternate
screen when it is dropped. To read numeric codes for arrows and mouse events,
use `Terminal::get_key(timeout_ms, wait)`.
