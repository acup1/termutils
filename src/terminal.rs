use libc::{
    ioctl, poll, pollfd, read, tcgetattr, tcsetattr, termios as Termios, winsize, ECHO, ICANON,
    POLLIN, STDOUT_FILENO, TCSANOW, TIOCGWINSZ, VMIN, VTIME,
};
use std::io::{self, Stdin, Stdout, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const KEY_FAIL: i32 = 0;
pub const KEY_LEFT: i32 = 128;
pub const KEY_RIGHT: i32 = 129;
pub const KEY_UP: i32 = 130;
pub const KEY_DOWN: i32 = 131;
pub const MOUSE_EVENT: i32 = 132;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct MouseModifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl MouseModifiers {
    fn from_button_code(button: u16) -> Self {
        Self {
            shift: button & 4 != 0,
            alt: button & 8 != 0,
            ctrl: button & 16 != 0,
        }
    }
}

/// Controls which mouse events the terminal reports.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum MouseTrackingMode {
    /// Report button presses and releases, but no pointer movement.
    Normal,
    /// Report pointer movement while a button is held.
    #[default]
    ButtonEvent,
    /// Report every pointer movement, including movement without a held button.
    AnyEvent,
}

impl MouseTrackingMode {
    fn enable_sequence(self) -> &'static [u8] {
        match self {
            Self::Normal => b"\x1b[?1000h\x1b[?1006h",
            Self::ButtonEvent => b"\x1b[?1002h\x1b[?1006h",
            Self::AnyEvent => b"\x1b[?1003h\x1b[?1006h",
        }
    }
}

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
    pub modifiers: MouseModifiers,
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
        let mut original: Termios = unsafe { std::mem::zeroed() };
        if unsafe { tcgetattr(stdin_fd, &mut original) } == -1 {
            return Err(io::Error::last_os_error());
        }
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
        set_terminal_attributes(self.stdin_fd, &self.raw)?;
        self.mode_state = true;
        Ok(())
    }

    pub fn disable_raw_mode(&mut self) -> io::Result<()> {
        set_terminal_attributes(self.stdin_fd, &self.original)?;
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
        self.enable_mouse_tracking(MouseTrackingMode::ButtonEvent)
    }

    /// Enables mouse reporting at the requested granularity.
    ///
    /// [`MouseTrackingMode::AnyEvent`] asks compatible terminals to report
    /// cell coordinates continuously as the pointer moves. This uses terminal
    /// escape sequences only; no window-system integration is required.
    pub fn enable_mouse_tracking(&mut self, mode: MouseTrackingMode) -> io::Result<()> {
        self.stdout.write_all(
            b"\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l",
        )?;
        self.stdout.write_all(mode.enable_sequence())?;
        self.mouse_enabled = true;
        *self.mouse.lock().unwrap() = Mouse::default();
        self.refresh()
    }

    pub fn disable_mouse(&mut self) -> io::Result<()> {
        self.stdout
            .write_all(b"\x1b[?1006l\x1b[?1003l\x1b[?1002l\x1b[?1000l")?;
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
        let button = input[3].saturating_sub(32);
        let mouse = Mouse {
            x: input[4].saturating_sub(33) as u16,
            y: input[5].saturating_sub(33) as u16,
            event: mouse_event(u16::from(button), false),
            modifiers: MouseModifiers::from_button_code(u16::from(button)),
        };
        return (MOUSE_EVENT, Some(mouse));
    }
    if let Some(mouse) = parse_sgr_mouse(input) {
        return (MOUSE_EVENT, Some(mouse));
    }
    (input[0] as i32, None)
}

fn mouse_event(button: u16, released: bool) -> MouseEvent {
    if released {
        return MouseEvent::IDLE;
    }
    let base_button = button & 3;
    if button & 64 != 0 {
        return match base_button {
            0 => MouseEvent::WHEELUP,
            1 => MouseEvent::WHEELDOWN,
            _ => MouseEvent::IDLE,
        };
    }
    if button & 32 != 0 {
        return match base_button {
            0 => MouseEvent::SPAMLEFT,
            1 => MouseEvent::SPAMMIDDLE,
            2 => MouseEvent::SPAMRIGHT,
            _ => MouseEvent::IDLE,
        };
    }
    match base_button {
        0 => MouseEvent::LEFT,
        1 => MouseEvent::MIDDLE,
        2 => MouseEvent::RIGHT,
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
    Some(Mouse {
        x,
        y,
        event: mouse_event(button, released),
        modifiers: MouseModifiers::from_button_code(button),
    })
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore();
        let _ = set_terminal_attributes(self.stdin_fd, &self.original);
    }
}

fn set_terminal_attributes(fd: RawFd, attributes: &Termios) -> io::Result<()> {
    if unsafe { tcsetattr(fd, TCSANOW, attributes) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
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
                    modifiers: MouseModifiers::default(),
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
                    modifiers: MouseModifiers::default(),
                })
            )
        );
        assert_eq!(
            decode_input(b"\x1b[<0;12;9m").1.unwrap().event,
            MouseEvent::IDLE
        );
    }

    #[test]
    fn decodes_mouse_modifiers_without_changing_the_event() {
        let shifted_click = decode_input(b"\x1b[<4;12;9M").1.unwrap();
        assert_eq!(shifted_click.event, MouseEvent::LEFT);
        assert_eq!(
            shifted_click.modifiers,
            MouseModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            }
        );

        let modified_drag = decode_input(b"\x1b[<60;20;10M").1.unwrap();
        assert_eq!(modified_drag.event, MouseEvent::SPAMLEFT);
        assert_eq!(
            modified_drag.modifiers,
            MouseModifiers {
                shift: true,
                alt: true,
                ctrl: true,
            }
        );

        let shifted_x10_click = decode_input(&[27, b'[', b'M', 36, 38, 40]).1.unwrap();
        assert_eq!(shifted_x10_click.event, MouseEvent::LEFT);
        assert!(shifted_x10_click.modifiers.shift);
    }

    #[test]
    fn decodes_unpressed_pointer_motion() {
        let motion = decode_input(b"\x1b[<35;42;17M").1.unwrap();
        assert_eq!((motion.x, motion.y), (41, 16));
        assert_eq!(motion.event, MouseEvent::IDLE);
        assert_eq!(motion.modifiers, MouseModifiers::default());
    }

    #[test]
    fn mouse_tracking_modes_use_terminal_only_escape_sequences() {
        assert_eq!(
            MouseTrackingMode::Normal.enable_sequence(),
            b"\x1b[?1000h\x1b[?1006h"
        );
        assert_eq!(
            MouseTrackingMode::ButtonEvent.enable_sequence(),
            b"\x1b[?1002h\x1b[?1006h"
        );
        assert_eq!(
            MouseTrackingMode::AnyEvent.enable_sequence(),
            b"\x1b[?1003h\x1b[?1006h"
        );
    }
}
