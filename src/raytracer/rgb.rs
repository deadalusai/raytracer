
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb { 
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Rgb {
    pub fn new (r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r: r, g: g, b: b }
    }
}