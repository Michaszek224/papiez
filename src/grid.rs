use crate::color::Rgb;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Rgb,
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub w: u16,
    pub h: u16,
    pub cells: Vec<Cell>,
}

impl Grid {
    pub fn blank(w: u16, h: u16, fg: Rgb, bg: Rgb) -> Grid {
        Grid { w, h, cells: vec![Cell { ch: ' ', fg, bg }; w as usize * h as usize] }
    }
}
