use std::io::{self, Write};

use crate::color::Rgb;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct CellStyle {
    pub foreground: Option<Rgb>,
    pub background: Option<Rgb>,
}

impl CellStyle {
    pub const fn new(foreground: Option<Rgb>, background: Option<Rgb>) -> Self {
        Self {
            foreground,
            background,
        }
    }

    pub const fn foreground(color: Rgb) -> Self {
        Self::new(Some(color), None)
    }

    pub const fn background(color: Rgb) -> Self {
        Self::new(None, Some(color))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FrameCell {
    pub symbol: char,
    pub style: CellStyle,
}

impl Default for FrameCell {
    fn default() -> Self {
        Self {
            symbol: ' ',
            style: CellStyle::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct PresentStats {
    pub changed_cells: usize,
    pub cursor_moves: usize,
}

#[derive(Debug, Default)]
pub struct FrameBuffer {
    width: u16,
    height: u16,
    previous: Vec<FrameCell>,
    current: Vec<FrameCell>,
    valid: bool,
}

impl FrameBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        let mut buffer = Self::default();
        buffer.resize(width, height);
        buffer
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        let length = usize::from(width) * usize::from(height);
        self.previous = vec![FrameCell::default(); length];
        self.current = vec![FrameCell::default(); length];
        self.valid = false;
    }

    pub fn begin_frame(&mut self) {
        self.current.fill(FrameCell::default());
    }

    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    pub fn put(&mut self, y: i32, x: i32, symbol: char) -> bool {
        self.put_styled(y, x, symbol, CellStyle::default())
    }

    pub fn put_styled(&mut self, y: i32, x: i32, symbol: char, style: CellStyle) -> bool {
        let Some(index) = self.index(y, x) else {
            return false;
        };
        self.current[index] = FrameCell { symbol, style };
        true
    }

    pub fn put_text(&mut self, y: i32, x: i32, text: impl AsRef<str>) {
        self.put_text_styled(y, x, text, CellStyle::default());
    }

    pub fn put_text_styled(&mut self, y: i32, x: i32, text: impl AsRef<str>, style: CellStyle) {
        for (offset, symbol) in text.as_ref().chars().enumerate() {
            self.put_styled(y, x + offset as i32, symbol, style);
        }
    }

    pub fn get(&self, y: i32, x: i32) -> Option<FrameCell> {
        self.index(y, x).map(|index| self.current[index])
    }

    pub fn present(&mut self, writer: &mut impl Write) -> io::Result<PresentStats> {
        let mut stats = PresentStats::default();
        let width = usize::from(self.width);
        let mut active_style = CellStyle::default();

        for y in 0..usize::from(self.height) {
            let mut x = 0;
            while x < width {
                while x < width && !self.changed(y * width + x) {
                    x += 1;
                }
                if x == width {
                    break;
                }

                let run_start = x;
                while x < width && self.changed(y * width + x) {
                    x += 1;
                }

                write!(writer, "\x1b[{};{}H", y + 1, run_start + 1)?;
                stats.cursor_moves += 1;
                stats.changed_cells += x - run_start;
                for column in run_start..x {
                    let cell = self.current[y * width + column];
                    if cell.style != active_style {
                        write_style(writer, cell.style, active_style != CellStyle::default())?;
                        active_style = cell.style;
                    }
                    let symbol = cell.symbol;
                    write!(writer, "{symbol}")?;
                }
            }
        }

        if active_style != CellStyle::default() {
            writer.write_all(b"\x1b[0m")?;
        }

        writer.flush()?;
        self.previous.clone_from(&self.current);
        self.valid = true;
        Ok(stats)
    }

    fn changed(&self, index: usize) -> bool {
        !self.valid || self.previous[index] != self.current[index]
    }

    fn index(&self, y: i32, x: i32) -> Option<usize> {
        if y < 0 || x < 0 || y >= i32::from(self.height) || x >= i32::from(self.width) {
            return None;
        }
        Some(y as usize * usize::from(self.width) + x as usize)
    }
}

fn write_style(writer: &mut impl Write, style: CellStyle, reset_previous: bool) -> io::Result<()> {
    if reset_previous {
        writer.write_all(b"\x1b[0m")?;
    }
    if let Some(color) = style.foreground {
        write!(writer, "\x1b[38;2;{};{};{}m", color.r, color.g, color.b)?;
    }
    if let Some(color) = style.background {
        write!(writer, "\x1b[48;2;{};{};{}m", color.r, color.g, color.b)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_frame_writes_nothing() {
        let mut frame = FrameBuffer::new(4, 2);
        frame.begin_frame();
        frame.put_text(0, 1, "ab");
        frame.present(&mut Vec::new()).unwrap();

        frame.begin_frame();
        frame.put_text(0, 1, "ab");
        let mut output = Vec::new();
        let stats = frame.present(&mut output).unwrap();

        assert!(output.is_empty());
        assert_eq!(stats, PresentStats::default());
    }

    #[test]
    fn contiguous_changes_share_one_cursor_move() {
        let mut frame = FrameBuffer::new(6, 1);
        frame.begin_frame();
        frame.present(&mut Vec::new()).unwrap();

        frame.begin_frame();
        frame.put_text(0, 2, "abc");
        let mut output = Vec::new();
        let stats = frame.present(&mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "\x1b[1;3Habc");
        assert_eq!(stats.changed_cells, 3);
        assert_eq!(stats.cursor_moves, 1);
    }

    #[test]
    fn separated_changes_use_the_minimum_changed_only_runs() {
        let mut frame = FrameBuffer::new(5, 1);
        frame.begin_frame();
        frame.present(&mut Vec::new()).unwrap();

        frame.begin_frame();
        frame.put(0, 0, 'a');
        frame.put(0, 4, 'b');
        let mut output = Vec::new();
        let stats = frame.present(&mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "\x1b[1;1Ha\x1b[1;5Hb");
        assert_eq!(stats.changed_cells, 2);
        assert_eq!(stats.cursor_moves, 2);
    }

    #[test]
    fn removed_content_is_erased_with_spaces() {
        let mut frame = FrameBuffer::new(3, 1);
        frame.begin_frame();
        frame.put_text(0, 0, "abc");
        frame.present(&mut Vec::new()).unwrap();

        frame.begin_frame();
        let mut output = Vec::new();
        let stats = frame.present(&mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "\x1b[1;1H   ");
        assert_eq!(stats.changed_cells, 3);
        assert_eq!(stats.cursor_moves, 1);
    }

    #[test]
    fn style_changes_are_part_of_the_frame_diff() {
        let mut frame = FrameBuffer::new(1, 1);
        frame.begin_frame();
        frame.put_styled(
            0,
            0,
            'x',
            CellStyle::new(Some(Rgb::new(255, 0, 0)), Some(Rgb::new(0, 0, 255))),
        );
        frame.present(&mut Vec::new()).unwrap();

        frame.begin_frame();
        frame.put_styled(0, 0, 'x', CellStyle::foreground(Rgb::new(0, 255, 0)));
        let mut output = Vec::new();
        let stats = frame.present(&mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\x1b[1;1H\x1b[38;2;0;255;0mx\x1b[0m"
        );
        assert_eq!(stats.changed_cells, 1);
        assert_eq!(stats.cursor_moves, 1);
    }
}
