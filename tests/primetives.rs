use std::io;

use termutils::{primetives::Primetive, terminal};

fn rect() -> io::Result<()> {
    let mut t = terminal::Terminal::wannajustwork()?;

    t.setpos(10, 10);
    let mut rect = Primetive::new(termutils::primetives::PrimetiveType::RECT, 0, 0, 20, 10);
    rect.draw();
    // Primetive::test();

    t.refresh()?;

    loop {
        match t.getch(100) {
            'q' => break,
            _ => {}
        }
    }
    t.restoreall();
    Ok(())
}

#[test]
#[ignore = "requires an interactive terminal"]
fn primetives_test() -> io::Result<()> {
    rect()?;
    Ok(())
}
