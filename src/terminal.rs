use libc::{ioctl, poll, pollfd, read, winsize, POLLIN, STDOUT_FILENO, TIOCGWINSZ};
use std::io::{self, Stdin, Stdout, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use termios::{tcsetattr, Termios, ECHO, ICANON, TCSANOW, VMIN, VTIME};

pub const KEY_FAIL: i32 = 0;
pub const KEY_LEFT: i32 = 128;
pub const KEY_RIGHT: i32 = 129;
pub const KEY_UP: i32 = 130;
pub const KEY_DOWN: i32 = 131;
pub const MOUSE_EVENT: i32 = 132;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum MouseEvent {
    #[default]
    IDLE,
    LEFT,
    SPAMLEFT,
    MIDDLE,
    SPAMMIDDLE,
    RIGHT,
    SPAMRIGHT,
    WHEELUP,
    WHEELDOWN,
}

impl MouseEvent {
    pub const SPAM_LEFT: Self = Self::SPAMLEFT;
    pub const SPAM_MIDDLE: Self = Self::SPAMMIDDLE;
    pub const SPAM_RIGHT: Self = Self::SPAMRIGHT;
    pub const WHEEL_UP: Self = Self::WHEELUP;
    pub const WHEEL_DOWN: Self = Self::WHEELDOWN;
}

impl From<MouseEvent> for u8 {
    fn from(event: MouseEvent) -> Self {
        match event {
            MouseEvent::IDLE => 0,
            MouseEvent::LEFT => 1,
            MouseEvent::SPAMLEFT => 2,
            MouseEvent::RIGHT => 3,
            MouseEvent::SPAMRIGHT => 4,
            MouseEvent::MIDDLE => 5,
            MouseEvent::SPAMMIDDLE => 6,
            MouseEvent::WHEELUP => 7,
            MouseEvent::WHEELDOWN => 8,
        }
    }
}

impl From<u8> for MouseEvent {
    fn from(event: u8) -> Self {
        match event {
            1 => MouseEvent::LEFT,
            2 => MouseEvent::SPAMLEFT,
            3 => MouseEvent::RIGHT,
            4 => MouseEvent::SPAMRIGHT,
            5 => MouseEvent::MIDDLE,
            6 => MouseEvent::SPAMMIDDLE,
            7 => MouseEvent::WHEELUP,
            8 => MouseEvent::WHEELDOWN,
            _ => MouseEvent::IDLE,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Mouse {
    pub x: u16,
    pub y: u16,
    pub event: MouseEvent,
}

pub struct Terminal {
    original: Termios,
    raw: Termios,
    pub mode_state: bool,
    pub alt_buffer: bool,
    pub mouse_enabled: bool,
    cursor_visible: bool,
    pub width: u16,
    pub height: u16,
    pub x: u16,
    pub y: u16,
    pub stdin: Stdin,
    pub stdout: Stdout,
    stdin_fd: RawFd,
    // Compatibility mirrors of the latest synchronous read.
    pub readed: Arc<Mutex<usize>>,
    pub reading_buffer: Arc<Mutex<[u8; 20]>>,
    pub key_pressed: Arc<Mutex<char>>,
    pub mouse: Arc<Mutex<Mouse>>,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        let stdin = io::stdin();
        let stdin_fd = stdin.as_raw_fd();
        let original = Termios::from_fd(stdin_fd)?;
        let mut raw = original;
        raw.c_lflag &= !(ICANON | ECHO);
        raw.c_cc[VMIN] = 1;
        raw.c_cc[VTIME] = 0;

        let mut terminal = Self {
            original,
            raw,
            mode_state: false,
            alt_buffer: false,
            mouse_enabled: false,
            cursor_visible: true,
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            stdin,
            stdout: io::stdout(),
            stdin_fd,
            readed: Arc::new(Mutex::new(0)),
            reading_buffer: Arc::new(Mutex::new([0; 20])),
            key_pressed: Arc::new(Mutex::new('\0')),
            mouse: Arc::new(Mutex::new(Mouse::default())),
        };
        terminal.update_size();
        Ok(terminal)
    }

    pub fn wannajustwork() -> io::Result<Self> {
        let mut terminal = Self::new()?;
        terminal.enable_raw_mode()?;
        terminal.enable_alt_buffer()?;
        terminal.enable_mouse()?;
        terminal.clear()?;
        terminal.curset(false)?;
        terminal.setpos(0, 0);
        Ok(terminal)
    }

    pub fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;
        if let Err(error) = self.stdout.write_all(b"\x1b[0m") {
            first_error = Some(error);
        }
        if self.mouse_enabled {
            if let Err(error) = self.disable_mouse() {
                first_error.get_or_insert(error);
            }
        }
        if !self.cursor_visible {
            if let Err(error) = self.curset(true) {
                first_error.get_or_insert(error);
            }
        }
        if self.alt_buffer {
            if let Err(error) = self.disable_alt_buffer() {
                first_error.get_or_insert(error);
            }
        }
        if self.mode_state {
            if let Err(error) = self.disable_raw_mode() {
                first_error.get_or_insert(error);
            }
        }
        if let Err(error) = self.stdout.flush() {
            first_error.get_or_insert(error);
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub fn restoreall(&mut self) {
        let _ = self.restore();
    }

    fn update_size(&mut self) {
        let mut size: winsize = unsafe { std::mem::zeroed() };
        if unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size) } != -1 {
            self.width = size.ws_col;
            self.height = size.ws_row;
        }
    }

    pub fn getsize(&mut self) -> (u16, u16) {
        self.update_size();
        (self.height, self.width)
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        self.stdout.flush()?;
        self.update_size();
        Ok(())
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.stdout.write_all(b"\x1b[2J")?;
        self.refresh()
    }

    pub fn curset(&mut self, visibility: bool) -> io::Result<()> {
        self.stdout
            .write_all(if visibility { b"\x1b[?25h" } else { b"\x1b[?25l" })?;
        self.cursor_visible = visibility;
        self.refresh()
    }

    fn write_position(&mut self, y: u16, x: u16) -> io::Result<()> {
        write!(self.stdout, "\x1b[{};{}H", y + 1, x + 1)?;
        self.x = x;
        self.y = y;
        Ok(())
    }

    pub fn setpos(&mut self, y: u16, x: u16) {
        if (self.height == 0 || y < self.height) && (self.width == 0 || x < self.width) {
            let _ = self.write_position(y, x);
        }
    }

    pub fn posprint(&mut self, y: u16, x: u16, text: impl AsRef<str>) {
        if (self.height == 0 || y < self.height) && (self.width == 0 || x < self.width) {
            if self.write_position(y, x).is_ok() {
                let _ = self.stdout.write_all(text.as_ref().as_bytes());
            }
        }
    }

    pub fn poschar(&mut self, y: u16, x: u16, character: char) {
        self.poswchar(y, x, character);
    }

    pub fn poswchar(&mut self, y: u16, x: u16, character: char) {
        let mut encoded = [0; 4];
        self.posprint(y, x, character.encode_utf8(&mut encoded));
    }

    pub fn draw_box(&mut self, y: u16, x: u16, height: u16, width: u16, square: bool) {
        if height < 2 || width < 2 {
            return;
        }
        let (top_left, top_right, bottom_left, bottom_right) = if square {
            ('┌', '┐', '└', '┘')
        } else {
            ('╭', '╮', '╰', '╯')
        };
        for column in 1..width - 1 {
            self.poswchar(y, x.saturating_add(column), '─');
            self.poswchar(y.saturating_add(height - 1), x.saturating_add(column), '─');
        }
        for row in 1..height - 1 {
            self.poswchar(y.saturating_add(row), x, '│');
            self.poswchar(y.saturating_add(row), x.saturating_add(width - 1), '│');
        }
        self.poswchar(y, x, top_left);
        self.poswchar(y, x.saturating_add(width - 1), top_right);
        self.poswchar(y.saturating_add(height - 1), x, bottom_left);
        self.poswchar(
            y.saturating_add(height - 1),
            x.saturating_add(width - 1),
            bottom_right,
        );
    }

    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
        tcsetattr(self.stdin_fd, TCSANOW, &self.raw)?;
        self.mode_state = true;
        Ok(())
    }

    pub fn disable_raw_mode(&mut self) -> io::Result<()> {
        tcsetattr(self.stdin_fd, TCSANOW, &self.original)?;
        self.mode_state = false;
        Ok(())
    }

    pub fn switch_raw_mode(&mut self) -> io::Result<()> {
        if self.mode_state {
            self.disable_raw_mode()
        } else {
            self.enable_raw_mode()
        }
    }

    pub fn enable_alt_buffer(&mut self) -> io::Result<()> {
        self.stdout.write_all(b"\x1b[?1049h")?;
        self.alt_buffer = true;
        self.refresh()
    }

    pub fn disable_alt_buffer(&mut self) -> io::Result<()> {
        self.stdout.write_all(b"\x1b[?1049l")?;
        self.alt_buffer = false;
        self.refresh()
    }

    pub fn switch_alt_buffer(&mut self) -> io::Result<()> {
        if self.alt_buffer {
            self.disable_alt_buffer()
        } else {
            self.enable_alt_buffer()
        }
    }

    pub fn enable_mouse(&mut self) -> io::Result<()> {
        self.stdout
            .write_all(b"\x1b[?1000h\x1b[?1002h\x1b[?1006h")?;
        self.mouse_enabled = true;
        *self.mouse.lock().unwrap() = Mouse::default();
        self.refresh()
    }

    pub fn disable_mouse(&mut self) -> io::Result<()> {
        self.stdout
            .write_all(b"\x1b[?1006l\x1b[?1002l\x1b[?1000l")?;
        self.mouse_enabled = false;
        self.refresh()
    }

    pub fn switch_mouse(&mut self) -> io::Result<()> {
        if self.mouse_enabled {
            self.disable_mouse()
        } else {
            self.enable_mouse()
        }
    }

    pub fn get_key(&mut self, timeout_ms: u64, wait: bool) -> io::Result<i32> {
        let started = Instant::now();
        let mut descriptor = pollfd {
            fd: self.stdin_fd,
            events: POLLIN,
            revents: 0,
        };
        let poll_timeout = timeout_ms.min(i32::MAX as u64) as i32;
        let poll_result = unsafe { poll(&mut descriptor, 1, poll_timeout) };
        if poll_result < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut input = [0u8; 20];
        let count = if poll_result == 0 || descriptor.revents & POLLIN == 0 {
            0
        } else {
            let count = unsafe { read(self.stdin_fd, input.as_mut_ptr().cast(), input.len()) };
            if count < 0 {
                return Err(io::Error::last_os_error());
            }
            count as usize
        };

        *self.reading_buffer.lock().unwrap() = input;
        *self.readed.lock().unwrap() = count;
        let (key, mouse) = decode_input(&input[..count]);
        if let Some(mouse) = mouse {
            *self.mouse.lock().unwrap() = mouse;
        }
        *self.key_pressed.lock().unwrap() = if (1..=127).contains(&key) {
            char::from_u32(key as u32).unwrap_or('\0')
        } else {
            '\0'
        };

        if wait {
            let frame = Duration::from_millis(timeout_ms);
            if let Some(remaining) = frame.checked_sub(started.elapsed()) {
                thread::sleep(remaining);
            }
        }
        Ok(key)
    }

    pub fn getch(&mut self, timeout_ms: u64) -> char {
        self.get_key(timeout_ms, false)
            .ok()
            .and_then(|key| char::from_u32(key as u32))
            .unwrap_or('\0')
    }

    pub fn getmouse(&self) -> Mouse {
        *self.mouse.lock().unwrap()
    }
}

fn decode_input(input: &[u8]) -> (i32, Option<Mouse>) {
    if input.is_empty() {
        return (KEY_FAIL, None);
    }
    if input.starts_with(b"\x1b[A") {
        return (KEY_UP, None);
    }
    if input.starts_with(b"\x1b[B") {
        return (KEY_DOWN, None);
    }
    if input.starts_with(b"\x1b[C") {
        return (KEY_RIGHT, None);
    }
    if input.starts_with(b"\x1b[D") {
        return (KEY_LEFT, None);
    }
    if input.len() >= 6 && input.starts_with(b"\x1b[M") {
        let mouse = Mouse {
            x: input[4].saturating_sub(33) as u16,
            y: input[5].saturating_sub(33) as u16,
            event: x10_mouse_event(input[3]),
        };
        return (MOUSE_EVENT, Some(mouse));
    }
    if let Some(mouse) = parse_sgr_mouse(input) {
        return (MOUSE_EVENT, Some(mouse));
    }
    (input[0] as i32, None)
}

fn x10_mouse_event(button: u8) -> MouseEvent {
    match button {
        32 => MouseEvent::LEFT,
        33 => MouseEvent::MIDDLE,
        34 => MouseEvent::RIGHT,
        35 => MouseEvent::IDLE,
        64 => MouseEvent::SPAMLEFT,
        65 => MouseEvent::SPAMMIDDLE,
        66 => MouseEvent::SPAMRIGHT,
        96 => MouseEvent::WHEELUP,
        97 => MouseEvent::WHEELDOWN,
        _ => MouseEvent::IDLE,
    }
}

fn parse_sgr_mouse(input: &[u8]) -> Option<Mouse> {
    let body = input.strip_prefix(b"\x1b[<")?;
    let end = body.iter().position(|byte| *byte == b'M' || *byte == b'm')?;
    let released = body[end] == b'm';
    let values = std::str::from_utf8(&body[..end]).ok()?;
    let mut values = values.split(';').map(str::parse::<u16>);
    let button = values.next()?.ok()?;
    let x = values.next()?.ok()?.saturating_sub(1);
    let y = values.next()?.ok()?.saturating_sub(1);
    if values.next().is_some() {
        return None;
    }
    let event = if released {
        MouseEvent::IDLE
    } else {
        match button {
            0 => MouseEvent::LEFT,
            1 => MouseEvent::MIDDLE,
            2 => MouseEvent::RIGHT,
            32 => MouseEvent::SPAMLEFT,
            33 => MouseEvent::SPAMMIDDLE,
            34 => MouseEvent::SPAMRIGHT,
            35 => MouseEvent::IDLE,
            64 => MouseEvent::WHEELUP,
            65 => MouseEvent::WHEELDOWN,
            _ => MouseEvent::IDLE,
        }
    };
    Some(Mouse { x, y, event })
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore();
        let _ = tcsetattr(self.stdin_fd, TCSANOW, &self.original);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_keys_and_both_mouse_protocols() {
        assert_eq!(decode_input(b"\x1b[A"), (KEY_UP, None));
        assert_eq!(decode_input(b"q"), ('q' as i32, None));
        assert_eq!(
            decode_input(&[27, b'[', b'M', 32, 38, 40]),
            (
                MOUSE_EVENT,
                Some(Mouse {
                    x: 5,
                    y: 7,
                    event: MouseEvent::LEFT,
                })
            )
        );
        assert_eq!(
            decode_input(b"\x1b[<64;12;9M"),
            (
                MOUSE_EVENT,
                Some(Mouse {
                    x: 11,
                    y: 8,
                    event: MouseEvent::WHEELUP,
                })
            )
        );
        assert_eq!(
            decode_input(b"\x1b[<0;12;9m").1.unwrap().event,
            MouseEvent::IDLE
        );
    }
}
