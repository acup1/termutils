use crate::cursor;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PrimetiveType {
    /// Square-cornered rectangle.
    RECT,
    /// Rounded-cornered rectangle.
    SRECT,
}

pub struct Primetive {
    pub drawing_type: PrimetiveType,
    pub x: u16,
    pub y: u16,
    pub dx: u16,
    pub dy: u16,
    layer: u16,
}

impl Primetive {
    pub fn new(drawing_type: PrimetiveType, x: u16, y: u16, dx: u16, dy: u16) -> Self {
        Primetive {
            drawing_type: drawing_type,
            x: x,
            y: y,
            dx: dx,
            dy: dy,
            layer: 0,
        }
    }
    pub fn draw(&mut self) {
        print!("\x1b[{};{}H", self.y + 1, self.x + 1);
        let width = self.dx;
        for row in self {
            print!("{}{}", row, cursor::translate(-(width as i16), 1));
        }
    }
}
impl Iterator for Primetive {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.dx < 2 || self.dy < 2 {
            self.layer = 0;
            return None;
        }

        let corners = match self.drawing_type {
            PrimetiveType::RECT => ('┌', '┐', '└', '┘'),
            PrimetiveType::SRECT => ('╭', '╮', '╰', '╯'),
        };
        let ret = match self.layer {
            0 => Some(format!(
                "{}{}{}",
                corners.0,
                "─".repeat(self.dx as usize - 2),
                corners.1
            )),
            n if n == self.dy - 1 => Some(format!(
                "{}{}{}",
                corners.2,
                "─".repeat(self.dx as usize - 2),
                corners.3
            )),
            n if n >= self.dy => None,
            _ => Some(format!("│{}│", cursor::right(self.dx - 2))),
        };
        self.layer = if self.layer >= self.dy { 0 } else { self.layer + 1 };
        ret
    }
}

/// Correctly-spelled aliases for new code; legacy names stay available.
pub type Primitive = Primetive;
pub type PrimitiveType = PrimetiveType;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_dimensions_and_styles_match_box_contract() {
        let square: Vec<_> = Primetive::new(PrimetiveType::RECT, 0, 0, 4, 3).collect();
        assert_eq!(square, ["┌──┐", "│\x1b[2C│", "└──┘"]);

        let rounded: Vec<_> = Primetive::new(PrimetiveType::SRECT, 0, 0, 3, 2).collect();
        assert_eq!(rounded, ["╭─╮", "╰─╯"]);
    }
}
