use std::{
    io::{self, Bytes},
    thread,
    time::Duration,
};

use termutils::{
    color::Color,
    terminal::{self, MouseEvent},
};

fn qexit() -> io::Result<()> {
    let mut t = terminal::Terminal::new()?;
    t.enable_raw_mode()?;

    while *t.key_pressed.lock().unwrap() != 'q' {
        t.setpos(0, 0);
    }

    t.disable_raw_mode()?;
    Ok(())
}

fn buffer_qexit() -> io::Result<()> {
    let mut t = terminal::Terminal::new()?;
    t.enable_raw_mode()?;
    t.enable_alt_buffer()?;
    t.clear()?;
    t.curset(false)?;
    let s: String = "press q in this buffer".to_string();
    t.posprint(t.height / 2, (t.width - (s.len() as u16)) / 2, s);
    t.refresh()?;

    while *t.key_pressed.lock().unwrap() != 'q' {
        t.setpos(0, 0);
    }

    t.disable_alt_buffer()?;
    t.disable_raw_mode()?;
    Ok(())
}

fn mouse_test() -> io::Result<()> {
    let mut t = terminal::Terminal::new()?;
    t.enable_raw_mode()?;
    t.enable_alt_buffer()?;
    t.enable_mouse()?;
    t.clear()?;
    t.curset(false)?;
    let s: String = "press q in this mouse test buffer".to_string();
    t.posprint(t.height / 2, (t.width - (s.len() as u16)) / 2, s);
    t.refresh()?;

    t.setpos(0, 0);
    let mut flag = false;
    loop {
        match t.getch(100) {
            'q' => break,
            'c' => t.clear()?,
            ' ' => flag = !flag,

            c => t.posprint(0, 0, format!("{}", c)),
            // _ => {}
        }
        let buff = *t.reading_buffer.lock().unwrap();
        t.clear()?;
        {
            let (x, y, event) = {
                let mouse_guard = t.mouse.lock().unwrap();
                (mouse_guard.x, mouse_guard.y, mouse_guard.event)
            };
            t.posprint(
                t.height / 2 - 1,
                0,
                format!("x={}, y={} --- {}", x, y, event as u8),
            );
        }

        if flag {
            let mouse_string = String::from_utf8_lossy(&buff);
            t.posprint(t.height / 2, 0, format!("{:?}", mouse_string));
            t.posprint(t.height / 2 + 1, 0, format!("{:?}", &buff));
        }

        // let mut m = t.mouse.lock().unwrap();
        // t.posprint(t.height / 2 + 2, 0, format!("Mouse {} {}", m.x, m.y));
        t.posprint(0, t.width / 2 - 2, format!("{}", flag));

        t.refresh()?;

        // for i in buff {
        //     print!("{} ", i as char);
        // }
        // println!();
        // t.refresh()?;
        // println!("{:?}", buff.map(|b| b as char));
    }

    t.disable_mouse()?;
    t.curset(true)?;
    t.disable_alt_buffer()?;
    t.disable_raw_mode()?;
    Ok(())
}

fn wannasimplicity() -> io::Result<()> {
    let mut t = terminal::Terminal::wannajustwork()?;
    t.refresh()?;
    t.curset(true);
    loop {
        match t.getch(2) {
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
            t.posprint(
                0,
                0,
                format!(
                    "{}{} {} {}{}",
                    Color::Green.fg(),
                    event as u8,
                    x,
                    y,
                    Color::reset()
                ),
            );
            match event {
                MouseEvent::LEFT => t.posprint(y, x, " ".to_string()),
                MouseEvent::SPAMLEFT => t.posprint(y, x, " ".to_string()),
                MouseEvent::WHEELUP => t.posprint(y, x, " ".to_string()),
                MouseEvent::WHEELDOWN => t.posprint(y, x, " ".to_string()),
                _ => {}
            }
        }
        t.refresh()?;
    }
    t.restoreall();
    Ok(())
}

#[test]
fn terminal_tests() -> io::Result<()> {
    qexit()?;
    buffer_qexit()?;
    mouse_test()?;
    wannasimplicity()?;
    Ok(())
}
