// #[allow(unused)]
use std::fmt;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn foreground(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    pub fn background(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }

    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    Grey,
    White,
    Reset,
}

impl From<Color> for u8 {
    fn from(color: Color) -> Self {
        match color {
            Color::Black => 0,
            Color::Red => 1,
            Color::Green => 2,
            Color::Yellow => 3,
            Color::Blue => 4,
            Color::Purple => 5,
            Color::Cyan => 6,
            Color::White => 7,
            Color::Grey => 8,
            Color::Reset => 9,
        }
    }
}
impl From<u8> for Color {
    fn from(color: u8) -> Self {
        match color {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Purple,
            6 => Color::Cyan,
            7 => Color::White,
            8 => Color::Grey,
            _ => Color::Reset,
        }
    }
}

impl Color {
    pub fn fg(self) -> String {
        match self {
            Color::Reset => "\x1b[0m".to_string(),
            Color::Grey => "\x1b[90m".to_string(),
            _ => format!("\x1b[{}m", 30 + u8::from(self)),
        }
    }

    pub fn bg(self) -> String {
        match self {
            Color::Reset => "\x1b[0m".to_string(),
            Color::Grey => "\x1b[100m".to_string(),
            _ => format!("\x1b[{}m", 40 + u8::from(self)),
        }
    }
    pub fn reset() -> String {
        "\x1b[0m".to_string()
    }
}

pub struct ColorPair<T, TT>
where
    T: Into<u8>,
    TT: Into<u8>,
{
    pub bg: T,
    pub fg: TT,
}
impl<T, TT> From<ColorPair<T, TT>> for String
where
    T: Into<u8>,
    TT: Into<u8>,
{
    fn from(color_pair: ColorPair<T, TT>) -> Self {
        format!(
            "{}{}",
            Color::from(color_pair.bg.into()).bg(),
            Color::from(color_pair.fg.into()).fg()
        )
    }
}
impl<T, TT> fmt::Display for ColorPair<T, TT>
where
    T: Into<u8> + Copy,
    TT: Into<u8> + Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}",
            Color::from(self.bg.into()).bg(),
            Color::from(self.fg.into()).fg()
        )
    }
}
