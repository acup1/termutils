use std::io;

use std::time::Duration;
use terminalib::{
    terminal::{self, MouseEvent},
    tgp::{query_support, ImageData, TerminalImage},
};

fn img_from_path() -> io::Result<()> {
    let mut t = terminal::Terminal::wannajustwork()?;

    if !query_support(&mut t, Duration::from_millis(250))? {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "terminal does not support the Kitty graphics protocol",
        ));
    }

    let data = ImageData::from_png_file("tests/imgs/err.png").map_err(io::Error::other)?;
    let mut img = TerminalImage::new(1, data).map_err(io::Error::other)?;
    img.upload(&mut t.stdout).map_err(io::Error::other)?;

    t.refresh()?;
    loop {
        match t.getch(10) {
            'q' => break,
            'c' => {
                t.clear()?;
                t.refresh()?;
            }
            _ => {}
        }
        {
            let (x, y, event) = {
                let mouse_guard = t.mouse.lock().unwrap();
                (mouse_guard.x, mouse_guard.y, mouse_guard.event)
            };
            match event {
                MouseEvent::LEFT => {
                    t.setpos(y, x);
                    img.place(&mut t.stdout, 20, 5).map_err(io::Error::other)?;
                }
                MouseEvent::SPAMLEFT => {
                    t.setpos(y, x);
                    img.place(&mut t.stdout, 20, 5).map_err(io::Error::other)?;
                }
                MouseEvent::WHEELUP => {
                    t.setpos(y, x);
                    img.place(&mut t.stdout, 20, 5).map_err(io::Error::other)?;
                }
                MouseEvent::WHEELDOWN => {
                    t.setpos(y, x);
                    img.place(&mut t.stdout, 20, 5).map_err(io::Error::other)?;
                }
                _ => {}
            }
        }
        t.refresh()?;
    }
    t.restoreall();
    Ok(())
}

#[test]
#[ignore = "requires an interactive terminal with Kitty graphics support"]
fn tgp_test() -> io::Result<()> {
    img_from_path()?;
    Ok(())
}
