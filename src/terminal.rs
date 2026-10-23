use libc::{ioctl, printf, winsize, STDOUT_FILENO, TIOCGWINSZ};
use std::io::{self, Read, Stdin, Stdout, Write};
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use termios::{tcgetattr, tcsetattr, Termios, ECHO, ICANON, TCSANOW, VMIN, VTIME};

#[derive(Clone, Copy, PartialEq)]
pub enum MouseEvent {
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
impl From<MouseEvent> for u8 {
    fn from(color: MouseEvent) -> Self {
        match color {
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
    fn from(color: u8) -> Self {
        match color {
            0 => MouseEvent::IDLE,
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

    pub width: u16,
    pub height: u16,

    pub x: u16,
    pub y: u16,

    pub stdin: Stdin,
    pub stdout: Stdout,
    stdin_fd: std::os::unix::io::RawFd,

    reading_thread_handle: Option<thread::JoinHandle<()>>,
    reading_thread_running: Arc<Mutex<bool>>,
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

        let mut raw = original.clone();
        raw.c_lflag &= !(ICANON | ECHO);
        raw.c_cc[VMIN] = 1;
        raw.c_cc[VTIME] = 0;

        let reading_thread_running = Arc::new(Mutex::new(true));
        let thread_reading_thread_running = reading_thread_running.clone();
        let buffer = Arc::new(Mutex::new([0u8; 20]));
        let thread_buffer = buffer.clone();
        let mouse = Arc::new(Mutex::new(Mouse {
            x: 0,
            y: 0,
            event: MouseEvent::IDLE,
        }));
        let thread_mouse = mouse.clone();
        let key_pressed = Arc::new(Mutex::new('\0'));
        let thread_key_pressed = key_pressed.clone();
        let readed = Arc::new(Mutex::new(0));
        let thread_readed = readed.clone();

        let reading_thread_handle = thread::spawn(move || {
            _ = Terminal::reading_thread_loop(
                thread_reading_thread_running,
                thread_buffer,
                thread_readed,
                thread_key_pressed,
                thread_mouse,
            );
        });

        Ok(Terminal {
            original: original,
            raw: raw,
            stdin: stdin,
            stdout: io::stdout(),
            stdin_fd: stdin_fd,
            mode_state: false,

            width: 0,
            height: 0,
            x: 0,
            y: 0,

            alt_buffer: false,
            mouse_enabled: false,

            reading_thread_handle: Some(reading_thread_handle),
            reading_thread_running: reading_thread_running,
            reading_buffer: buffer,
            readed: readed,
            key_pressed: key_pressed,

            mouse: mouse,
        })
    }
    pub fn wannajustwork() -> io::Result<Self> {
        let mut t = Self::new()?;
        t.enable_raw_mode()?;
        t.enable_alt_buffer()?;
        t.enable_mouse()?;
        t.clear()?;
        t.curset(false)?;
        t.setpos(0, 0);
        Ok(t)
    }
    pub fn restoreall(&mut self) {
        self.clear();
        self.refresh();
        self.disable_mouse();
        self.curset(true);
        self.disable_alt_buffer();
        self.disable_raw_mode();
    }

    fn stop_reading_thread_loop(&self) {
        *self.reading_thread_running.lock().unwrap() = false;
    }

    fn esc_reader(buff: &[u8], mouse: &mut Mouse) {
        let mouse_signature = [27, 91, 60];
        let pos = match buff
            .windows(mouse_signature.len())
            .position(|w| w == mouse_signature)
        {
            Some(a) => a + mouse_signature.len(),
            None => buff.len(),
        };
        let (mut button, mut x, mut y, mut action) = (0u16, 0u16, 0u16, ' ');
        let mut index = 0;
        for (i, byte) in buff.iter().enumerate() {
            if i >= pos {
                if *byte >= b'0' && *byte <= b'9' {
                    match index {
                        0 => button = button * 10 + (byte - b'0') as u16,
                        1 => x = x * 10 + (byte - b'0') as u16,
                        2 => y = y * 10 + (byte - b'0') as u16,
                        _ => {}
                    }
                } else if *byte == b';' {
                    index += 1;
                } else if *byte == b'M' || *byte == b'm' {
                    action = *byte as char;
                    break;
                }
            }
        }

        mouse.x = x;
        mouse.y = y;
        mouse.event = match (button, action) {
            (35, 'M') => MouseEvent::IDLE,
            (_, 'm') => MouseEvent::IDLE,
            (0, 'M') => MouseEvent::LEFT,
            (1, 'M') => MouseEvent::MIDDLE,
            (2, 'M') => MouseEvent::RIGHT,
            (32, 'M') => MouseEvent::SPAMLEFT,
            (33, 'M') => MouseEvent::SPAMMIDDLE,
            (34, 'M') => MouseEvent::SPAMRIGHT,
            (64, 'M') => MouseEvent::WHEELUP,
            (65, 'M') => MouseEvent::WHEELDOWN,
            (_, _) => MouseEvent::IDLE,
        };
    }

    fn reading_thread_loop(
        running: Arc<Mutex<bool>>,
        buffer: Arc<Mutex<[u8; 20]>>,
        readed: Arc<Mutex<usize>>,
        key_pressed: Arc<Mutex<char>>,
        mouse: Arc<Mutex<Mouse>>,
    ) -> io::Result<()> {
        let mut stdin = io::stdin();

        while *running.lock().unwrap() {
            let mut temp_buffer = [0u8; 20];
            match stdin.read(&mut temp_buffer) {
                Ok(0) => {
                    println!("0");
                }
                Ok(n) => {
                    *readed.lock().unwrap() = n;
                    let mut shared_key_pressed = key_pressed.lock().unwrap();
                    *shared_key_pressed = (temp_buffer)[0] as char;

                    let mut shared_buffer = buffer.lock().unwrap();
                    shared_buffer[..n].copy_from_slice(&temp_buffer[..n]);
                    if n < shared_buffer.len() {
                        shared_buffer[n..].fill(0);
                    }
                    if temp_buffer[0] == 0x1b {
                        let mut mouse_guard = mouse.lock().unwrap();
                        Terminal::esc_reader(&temp_buffer, &mut *mouse_guard);
                    }
                }
                Err(e) => eprintln!("Error reading: {}", e),
            }

            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    pub fn getch(&mut self, timeout: u128) -> char {
        let mut c: char = '\0';
        let start = Instant::now();
        while start.elapsed().as_micros() < timeout {
            let temp: char = *self.key_pressed.lock().unwrap();
            let mut n = self.readed.lock().unwrap();
            if temp != '\0' && *n > 0 {
                c = temp;
                *n = 0;
            }
        }
        c
    }
    pub fn getmouse(&mut self) -> Mouse {
        let guard = self.mouse.lock().unwrap();
        Mouse {
            x: guard.x,
            y: guard.y,
            event: guard.event.clone(),
        }
    }

    fn getsize(&mut self) {
        let mut w: winsize = unsafe { std::mem::zeroed() };
        if unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut w) } != -1 {
            self.width = w.ws_col;
            self.height = w.ws_row;
        }
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        self.stdout.flush()?;
        self.getsize();
        Ok(())
    }

    pub fn clear(&mut self) -> io::Result<()> {
        print!("\x1b[2J");
        self.refresh()?;
        Ok(())
    }

    pub fn curset(&mut self, visibility: bool) -> io::Result<()> {
        if visibility {
            print!("\x1b[?25h");
        } else {
            print!("\x1b[?25l");
        }
        self.refresh()?;
        Ok(())
    }
    pub fn setpos(&mut self, y: u16, x: u16) {
        print!("\x1b[{};{}H", y + 1, x + 1);
        self.x = x;
        self.y = y;
    }
    pub fn posprint(&mut self, y: u16, x: u16, s: String) {
        if y <= self.height && x <= self.width {
            self.setpos(y, x);
            _ = self.stdout.write(s.as_bytes());
        }
    }
    // void posprint(int y, int x, char *s) {
    // if (0 <= y && y <= ROWS && 0 <= x && x <= COLS) {
    // setpos(y, x);
    // printf("%s", s);
    // }
    // }

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
        match self.mode_state {
            false => self.enable_raw_mode()?,
            _ => self.disable_raw_mode()?,
        }
        Ok(())
    }

    pub fn enable_alt_buffer(&mut self) -> io::Result<()> {
        print!("\x1b[?1049h");
        self.alt_buffer = true;
        self.refresh()?;
        Ok(())
    }
    pub fn disable_alt_buffer(&mut self) -> io::Result<()> {
        print!("\x1b[?1049l");
        self.alt_buffer = false;
        self.refresh()?;
        Ok(())
    }
    pub fn switch_alt_buffer(&mut self) -> io::Result<()> {
        match self.alt_buffer {
            false => self.enable_alt_buffer()?,
            _ => self.disable_alt_buffer()?,
        }
        Ok(())
    }

    // \x1b[?1000h\x1b[?1003h\x1b[?1006h
    // \x1b[?1000l
    pub fn enable_mouse(&mut self) -> io::Result<()> {
        print!("\x1b[?1000h\x1b[?1003h\x1b[?1006h");
        self.mouse_enabled = true;
        self.refresh()?;
        Ok(())
    }
    pub fn disable_mouse(&mut self) -> io::Result<()> {
        print!("\x1b[?1000l\x1b[?1003l\x1b[?1006l");
        self.mouse_enabled = false;
        self.refresh()?;
        Ok(())
    }
    pub fn switch_mouse(&mut self) -> io::Result<()> {
        match self.mouse_enabled {
            false => self.enable_mouse()?,
            _ => self.disable_mouse()?,
        }
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.stop_reading_thread_loop();
        if let Some(handle) = self.reading_thread_handle.take() {
            handle.join().unwrap();
        }

        let _ = tcsetattr(self.stdin_fd, TCSANOW, &self.original);
    }
}
