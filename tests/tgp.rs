use std::io;

use termutils::{
    cursor,
    primetives::Primetive,
    terminal::{self, MouseEvent},
    tgp::ImgData,
};

fn img_from_path() -> io::Result<()> {
    let mut t = terminal::Terminal::wannajustwork()?;

    let mut img = ImgData::new_from_file(1, "tests/imgs/err.png", 240, 120);
    img.load();

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
                    img.draw(20, 5);
                }
                MouseEvent::SPAMLEFT => {
                    t.setpos(y, x);
                    img.draw(20, 5);
                }
                MouseEvent::WHEELUP => {
                    t.setpos(y, x);
                    img.draw(20, 5);
                }
                MouseEvent::WHEELDOWN => {
                    t.setpos(y, x);
                    img.draw(20, 5);
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
fn tgp_test() -> io::Result<()> {
    img_from_path()?;
    Ok(())
}
