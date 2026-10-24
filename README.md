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

## Framebuffer rendering

`WindowManager` composites every layer into a framebuffer. After the first
frame, it compares the new image with the displayed image and writes only
changed cells. Adjacent changed cells are emitted as a single run with one
cursor-position escape sequence.

Do not call `Terminal::clear` between managed frames: removed windows and cells
are erased by the framebuffer. If code writes directly to the terminal, call
`WindowManager::invalidate` before the next render. Use
`render_windows_with_stats` to inspect the number of changed cells and cursor
moves for a frame.

## Percentage layouts

`Layout` divides any terminal region into percentage-sized rows or columns and
can assign the resulting rectangles directly to windows. Percentages must total
100. Margins and gaps are removed before the remaining space is divided.

```rust,no_run
use termutils::{Layout, Rect, WindowManager};

let mut windows = WindowManager::new();
let sidebar = windows.new_window(0);
let content = windows.new_window(0);
let screen = Rect::from_terminal(24, 80);

let columns = Layout::horizontal([30, 70]).gap(1);
let regions = columns.apply(screen, &mut windows, &[sidebar, content])?;

// Layouts can be nested by splitting one of the returned regions again.
let content_rows = Layout::vertical([20, 80]).split(regions[1])?;
# Ok::<(), termutils::LayoutError>(())
```
