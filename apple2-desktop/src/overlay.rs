//! Generic on-screen overlay (OSD) framework, in the spirit of PicoApple2's
//! built-in menus. An overlay is an 80x48 text grid drawn with the Apple II
//! character ROM on top of a 2x-scaled, dimmed copy of the emulator frame.
//! Each concrete UI (memory monitor, later a disk picker, settings...) is a
//! `Screen`; the host (`Overlay`) owns the canvas, compositing and key routing.

use apple2_core::memory::Apple2Memory;
use apple2_core::video::{SCREEN_HEIGHT, SCREEN_WIDTH};
use minifb::Key;

pub const COLS: usize = 80;
pub const ROWS: usize = 48;
pub const WIDTH: usize = COLS * 7; // 560 = SCREEN_WIDTH * 2
pub const HEIGHT: usize = ROWS * 8; // 384 = SCREEN_HEIGHT * 2

// Shared palette so every screen looks like part of one system.
pub const BG: u32 = 0x00_10_14_10;
pub const TEXT: u32 = 0x00_D8_D8_D8;
pub const DIM: u32 = 0x00_70_78_70;
pub const ACCENT: u32 = 0x00_40_FF_40;
pub const HILITE: u32 = 0x00_FF_FF_40;
pub const ERROR: u32 = 0x00_FF_60_60;
pub const INVERSE_FG: u32 = 0x00_00_00_00;

#[derive(Clone, Copy)]
pub struct Cell {
    pub ch: u8,
    pub fg: u32,
    pub bg: u32,
}

/// Character grid. `None` cells are transparent (the dimmed emulator frame
/// shows through), so a screen can be a small popup or cover everything.
pub struct Canvas {
    cells: Vec<Option<Cell>>,
}

impl Canvas {
    pub fn new() -> Self {
        Self { cells: vec![None; COLS * ROWS] }
    }

    pub fn clear(&mut self) {
        self.cells.fill(None);
    }

    pub fn fill(&mut self, col: usize, row: usize, w: usize, h: usize, bg: u32) {
        for r in row..(row + h).min(ROWS) {
            for c in col..(col + w).min(COLS) {
                self.cells[r * COLS + c] = Some(Cell { ch: b' ', fg: TEXT, bg });
            }
        }
    }

    pub fn put(&mut self, col: usize, row: usize, ch: u8, fg: u32, bg: u32) {
        if col < COLS && row < ROWS {
            self.cells[row * COLS + col] = Some(Cell { ch, fg, bg });
        }
    }

    /// Writes `s` starting at (col,row), clipped at the right edge.
    /// Returns the column just past the text.
    pub fn text(&mut self, col: usize, row: usize, s: &str, fg: u32, bg: u32) -> usize {
        let mut c = col;
        for b in s.bytes() {
            self.put(c, row, b, fg, bg);
            c += 1;
        }
        c
    }

    #[cfg(test)]
    pub fn row_string(&self, row: usize) -> String {
        (0..COLS)
            .map(|c| self.cells[row * COLS + c].map_or(' ', |cell| cell.ch as char))
            .collect()
    }
}

pub enum Action {
    Stay,
    Close,
}

pub trait Screen {
    fn draw(&mut self, canvas: &mut Canvas, mem: &Apple2Memory);
    fn key(&mut self, key: Key, shift: bool, mem: &mut Apple2Memory) -> Action;
}

pub struct Overlay {
    canvas: Canvas,
    buffer: Vec<u32>,
    screen: Option<Box<dyn Screen>>,
}

impl Overlay {
    pub fn new() -> Self {
        Self { canvas: Canvas::new(), buffer: vec![0; WIDTH * HEIGHT], screen: None }
    }

    pub fn open(&mut self, screen: Box<dyn Screen>) {
        self.screen = Some(screen);
    }

    pub fn close(&mut self) {
        self.screen = None;
    }

    /// Routes a key-down to the active screen. Returns true if it closed.
    pub fn handle_key(&mut self, key: Key, shift: bool, mem: &mut Apple2Memory) -> bool {
        let Some(screen) = self.screen.as_mut() else { return false };
        if let Action::Close = screen.key(key, shift, mem) {
            self.screen = None;
            return true;
        }
        false
    }

    /// Composites the active screen over `frame` (SCREEN_WIDTH x SCREEN_HEIGHT)
    /// and returns a WIDTH x HEIGHT buffer ready for the window.
    pub fn render(&mut self, frame: &[u32], mem: &Apple2Memory, char_rom: &[u8; 2048]) -> &[u32] {
        // Background: emulator frame scaled 2x and dimmed to 1/4 brightness.
        for y in 0..HEIGHT {
            let src_row = &frame[(y / 2) * SCREEN_WIDTH..(y / 2 + 1) * SCREEN_WIDTH];
            let dst_row = &mut self.buffer[y * WIDTH..(y + 1) * WIDTH];
            for (x, px) in dst_row.iter_mut().enumerate() {
                *px = (src_row[x / 2] >> 2) & 0x00_3F_3F_3F;
            }
        }
        debug_assert_eq!(WIDTH, SCREEN_WIDTH * 2);
        debug_assert_eq!(HEIGHT, SCREEN_HEIGHT * 2);

        self.canvas.clear();
        if let Some(screen) = self.screen.as_mut() {
            screen.draw(&mut self.canvas, mem);
        }

        for row in 0..ROWS {
            for col in 0..COLS {
                let Some(cell) = self.canvas.cells[row * COLS + col] else { continue };
                let glyph = glyph_index(cell.ch) * 8;
                for gy in 0..8 {
                    let bits = char_rom[glyph + gy];
                    let base = (row * 8 + gy) * WIDTH + col * 7;
                    for gx in 0..7 {
                        let on = bits & (1 << (6 - gx)) != 0;
                        self.buffer[base + gx] = if on { cell.fg } else { cell.bg };
                    }
                }
            }
        }
        &self.buffer
    }
}

/// Character ROM slot for an ASCII byte: the II+ ROM's normal-video glyphs
/// live at $A0-$DF (ASCII | $80). It has no lowercase, so fold to uppercase.
fn glyph_index(ch: u8) -> usize {
    let c = match ch {
        b'a'..=b'z' => ch - 32,
        0x20..=0x5F => ch,
        _ => b'.',
    };
    (c | 0x80) as usize
}
