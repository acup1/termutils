# terminalib

A small Rust library for terminal raw mode, alternate-screen rendering, mouse
input, drawing primitives, and layered virtual windows.

```rust,no_run
use std::io;
use terminalib::{terminal::Terminal, WindowManager};

fn main() -> io::Result<()> {
    let mut terminal = Terminal::wannajustwork()?;
    let mut windows = WindowManager::new();
    let id = windows.new_window(0);

    let window = windows.get_mut(id).unwrap();
    window.x = 2;
    window.y = 1;
    window.width = 20;
    window.height = 5;
    window.name = Some("terminalib".into());
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

Mouse reports include `shift`, `alt`, and `ctrl` modifier flags. The default
`enable_mouse()` mode reports clicks and dragging. To receive cell coordinates
whenever the pointer moves, including without a pressed button, enable
terminal-only any-event tracking:

```rust,no_run
use terminalib::{terminal::Terminal, MouseTrackingMode};

let mut terminal = Terminal::new()?;
terminal.enable_mouse_tracking(MouseTrackingMode::AnyEvent)?;
terminal.get_key(100, false)?;

let mouse = terminal.getmouse();
println!("{}, {} (shift: {})", mouse.x, mouse.y, mouse.modifiers.shift);
# Ok::<(), std::io::Error>(())
```

Any-event tracking depends on xterm-compatible mouse support and reports
terminal cell coordinates, not pixels. It can generate a high volume of input.

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

## Terminal graphics protocol

The `tgp` module supports Kitty terminal graphics with active capability
detection, RGB/RGBA matrices, one-color masks, PNG files, and reusable terminal
image objects. Large payloads are split into protocol-safe chunks. Animation
frames can replace the root frame in-place or be uploaded for terminal-driven
playback.

```rust,no_run
use std::time::Duration;
use terminalib::{
    terminal::Terminal,
    tgp::{query_support, ImageData, ImageGroup, TerminalImage},
    Rgb,
};

let mut terminal = Terminal::new()?;
if query_support(&mut terminal, Duration::from_millis(500))? {
    let pixels = [[Rgb::new(255, 0, 0), Rgb::new(0, 0, 255)]];
    let mut image = TerminalImage::new(1, ImageData::from_rgb(pixels)?)?;
    image.upload_and_place(&mut terminal.stdout, 20, 10)?;

    // Grouping is explicit: all queued image updates are presented together.
    let mut group = ImageGroup::begin(&mut terminal.stdout)?;
    image.replace_frame(&mut group, ImageData::from_rgb(pixels)?)?;
    group.present()?;
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Run the interactive demo from the `examples` branch in a compatible terminal.
An optional argument loads another PNG instead of the bundled sample:

```sh
nix run github:acup1/terminalib/examples#tgp -- path/to/image.png
```

## Percentage layouts

`Layout` divides any terminal region into percentage-sized rows or columns and
can assign the resulting rectangles directly to windows. Percentages must total
100. Margins and gaps are removed before the remaining space is divided.

```rust,no_run
use terminalib::{Layout, Rect, WindowManager};

let mut windows = WindowManager::new();
let sidebar = windows.new_window(0);
let content = windows.new_window(0);
let screen = Rect::from_terminal(24, 80);

let columns = Layout::horizontal([30, 70]).gap(1);
let regions = columns.apply(screen, &mut windows, &[sidebar, content])?;

// Layouts can be nested by splitting one of the returned regions again.
let content_rows = Layout::vertical([20, 80]).split(regions[1])?;
# Ok::<(), terminalib::LayoutError>(())
```
