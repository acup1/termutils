use crate::cursor::{self, translate};

pub enum PrimetiveType {
    RECT,
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
        let dx = self.dx;
        for i in self {
            print!("{}{}", i, cursor::translate((dx as i16) * -1 - 1, 1));
        }
    }
}
impl Iterator for Primetive {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let ret: Option<String>;
        match self.layer {
            0 => ret = Some(format!("╭{}╮", "─".repeat(self.dx as usize - 1))),
            n if n == self.dy => ret = Some(format!("╰{}╯", "─".repeat(self.dx as usize - 1))),
            n if n > self.dy => ret = None,
            _ => ret = Some(format!("│{}│", cursor::right(self.dx - 1))),
        }
        if self.layer > self.dy {
            self.layer = 0;
        } else {
            self.layer += 1;
        }
        ret
    }
}
