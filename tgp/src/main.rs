use std::{error::Error, time::Duration};
use terminalib::{
    terminal::Terminal,
    tgp::{query_support, ImageData, TerminalImage},
    Rgb, Rgba,
};

const IMAGE_SIZE: usize = 64;
const SAMPLE_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
    0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn animated_frame(tick: u8) -> Result<ImageData, terminalib::tgp::GraphicsError> {
    let mut pixels = vec![vec![Rgb::BLACK; IMAGE_SIZE]; IMAGE_SIZE];
    for (y, row) in pixels.iter_mut().enumerate() {
        for (x, pixel) in row.iter_mut().enumerate() {
            *pixel = Rgb::new(
                (x as u8).wrapping_mul(4).wrapping_add(tick),
                (y as u8).wrapping_mul(4).wrapping_add(tick.wrapping_mul(2)),
                ((x + y) as u8)
                    .wrapping_mul(2)
                    .wrapping_add(tick.wrapping_mul(3)),
            );
        }
    }
    ImageData::from_rgb_rows(&pixels)
}

fn monochrome_logo() -> Result<ImageData, terminalib::tgp::GraphicsError> {
    let mut mask = vec![vec![false; 32]; 32];
    for (y, row) in mask.iter_mut().enumerate() {
        for (x, visible) in row.iter_mut().enumerate() {
            let border = !(4..28).contains(&x) || !(4..28).contains(&y);
            let diagonal = x.abs_diff(y) < 2 || x.abs_diff(31 - y) < 2;
            *visible = border || diagonal;
        }
    }
    ImageData::from_monochrome_rows(&mask, Rgba::new(80, 220, 140, 255))
}

fn png_image() -> Result<ImageData, terminalib::tgp::GraphicsError> {
    match std::env::args_os().nth(1) {
        Some(path) => ImageData::from_png_file(path),
        None => ImageData::from_png_bytes(SAMPLE_PNG),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = Terminal::new()?;
    if !query_support(&mut terminal, Duration::from_millis(500))? {
        eprintln!("This terminal does not support the Kitty graphics protocol.");
        return Ok(());
    }
    let (height, width) = terminal.getsize();
    if height < 15 || width < 68 {
        eprintln!("The TGP demo needs a terminal of at least 68x15 cells.");
        return Ok(());
    }

    terminal.enable_raw_mode()?;
    terminal.enable_alt_buffer()?;
    terminal.clear()?;
    terminal.curset(false)?;
    terminal.posprint(0, 0, "Terminalib TGP demo — q quits");
    terminal.posprint(1, 2, "per-pixel RGB / in-place frame replacement");
    terminal.posprint(1, 31, "one-color mask");
    terminal.posprint(1, 51, "PNG object");

    let mut animated = TerminalImage::new(1_001, animated_frame(0)?)?;
    terminal.setpos(2, 2);
    animated.upload_and_place(&mut terminal.stdout, 24, 12)?;

    let mut monochrome = TerminalImage::new(1_002, monochrome_logo()?)?;
    terminal.setpos(2, 31);
    monochrome.upload_and_place(&mut terminal.stdout, 16, 12)?;

    let mut png = TerminalImage::new(1_003, png_image()?)?;
    terminal.setpos(2, 51);
    png.upload_and_place(&mut terminal.stdout, 16, 12)?;
    terminal.refresh()?;

    let mut tick = 0u8;
    loop {
        if terminal.getch(33) == 'q' {
            break;
        }
        tick = tick.wrapping_add(3);
        animated.replace_frame(&mut terminal.stdout, animated_frame(tick)?)?;
        terminal.refresh()?;
    }

    animated.delete(&mut terminal.stdout)?;
    monochrome.delete(&mut terminal.stdout)?;
    png.delete(&mut terminal.stdout)?;
    terminal.refresh()?;
    Ok(())
}
