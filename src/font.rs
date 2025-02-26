#![allow(dead_code)]

#[derive(Clone, Copy)]
pub struct Glyph {
    pub w: u16,
    pub h: u16,
    pub l: i16,
    pub top: i16,
    pub adv: u16,
    pub offset: usize,
}

#[derive(Clone, Copy)]
pub struct Font<'a> {
    pub size: usize,
    pub ascent: i32,
    pub descent: i32,
    pub line_height: i32,
    pub first_char: usize,
    pub last_char: usize,
    pub glyphs: &'a [Glyph],
    pub pixels: &'a [u8],
    pub packed_len: usize,
    pub unpacked_len: usize,
}
