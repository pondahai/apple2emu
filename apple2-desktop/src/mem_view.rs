//! Overlay screen: cursor-driven hex editor over the Apple II address space.
//! Reads/writes go through `peek`/`poke`, so browsing never flips soft
//! switches and $D000+ edits land in the current Language Card bank.

use crate::overlay::{Action, Canvas, Screen, ACCENT, BG, COLS, DIM, ERROR, HILITE, INVERSE_FG, TEXT};
use apple2_core::memory::Apple2Memory;
use minifb::Key;

const BYTES_PER_ROW: u32 = 16;
const FIRST_DATA_ROW: usize = 3;
const DATA_ROWS: u32 = 42;
const PAGE: u32 = BYTES_PER_ROW * DATA_ROWS;
const HEX_COL: usize = 6;
const ASCII_COL: usize = HEX_COL + 16 * 3 + 2;
const STATUS_ROW: usize = 46;
const HELP_ROW: usize = 47;

pub struct MemView {
    cursor: u32,
    top: u32,
    /// View-only until the user explicitly enters edit mode (Enter).
    editing: bool,
    /// True after the first hex digit of a byte was typed.
    low_nibble: bool,
    goto_input: Option<String>,
    message: Option<(String, bool)>,
}

impl MemView {
    pub fn new(start: u16) -> Self {
        let mut v = Self { cursor: 0, top: 0, editing: false, low_nibble: false, goto_input: None, message: None };
        v.jump(start);
        v
    }

    /// Moves the cursor to `addr` with its row at the top of the page.
    fn jump(&mut self, addr: u16) {
        self.cursor = addr as u32;
        self.low_nibble = false;
        self.top = (self.cursor & !(BYTES_PER_ROW - 1)).min(0x1_0000 - PAGE);
    }

    fn scroll_to_cursor(&mut self) {
        let row_start = self.cursor & !(BYTES_PER_ROW - 1);
        if row_start < self.top {
            self.top = row_start;
        } else if row_start >= self.top + PAGE {
            self.top = row_start + BYTES_PER_ROW - PAGE;
        }
        self.top = self.top.min(0x1_0000 - PAGE);
    }

    fn move_cursor(&mut self, delta: i64) {
        self.cursor = (self.cursor as i64 + delta).clamp(0, 0xFFFF) as u32;
        self.low_nibble = false;
        self.scroll_to_cursor();
    }

    fn page(&mut self, dir: i64) {
        let delta = dir * PAGE as i64;
        self.top = (self.top as i64 + delta).clamp(0, (0x1_0000 - PAGE) as i64) as u32;
        self.move_cursor(delta);
    }

    fn edit(&mut self, digit: u8, mem: &mut Apple2Memory) {
        let addr = self.cursor as u16;
        let Some(old) = mem.peek(addr) else {
            self.message = Some((format!("${:04X} IS I/O SPACE - NOT EDITABLE", addr), true));
            return;
        };
        let new = if self.low_nibble { (old & 0xF0) | digit } else { (digit << 4) | (old & 0x0F) };
        if !mem.poke(addr, new) {
            self.message = Some((format!("${:04X} IS I/O SPACE - NOT EDITABLE", addr), true));
            return;
        }
        if addr >= 0xD000 && !mem.lc_read_enable {
            self.message = Some(("WROTE LC RAM - NOT VISIBLE WHILE ROM IS READ-ENABLED".into(), false));
        }
        if self.low_nibble {
            self.move_cursor(1);
        } else {
            self.low_nibble = true;
        }
    }

    fn goto_key(&mut self, key: Key) {
        let input = self.goto_input.as_mut().unwrap();
        match key {
            Key::Escape => self.goto_input = None,
            Key::Backspace => {
                input.pop();
            }
            Key::Enter | Key::NumPadEnter => {
                if let Ok(a) = u16::from_str_radix(input, 16) {
                    self.jump(a);
                }
                self.goto_input = None;
            }
            _ => {
                if let Some(d) = hex_digit(key) {
                    if input.len() < 4 {
                        input.push(char::from_digit(d as u32, 16).unwrap().to_ascii_uppercase());
                    }
                }
            }
        }
    }
}

impl Screen for MemView {
    fn key(&mut self, key: Key, _shift: bool, mem: &mut Apple2Memory) -> Action {
        if key == Key::F6 {
            return Action::Close;
        }
        if self.goto_input.is_some() {
            self.goto_key(key);
            return Action::Stay;
        }
        self.message = None;
        match key {
            Key::Escape if self.editing => {
                self.editing = false;
                self.low_nibble = false;
            }
            Key::Escape => return Action::Close,
            Key::Enter | Key::NumPadEnter => {
                self.editing = !self.editing;
                self.low_nibble = false;
            }
            Key::Left => self.move_cursor(-1),
            Key::Right => self.move_cursor(1),
            Key::Up => self.move_cursor(-(BYTES_PER_ROW as i64)),
            Key::Down => self.move_cursor(BYTES_PER_ROW as i64),
            Key::PageUp => self.page(-1),
            Key::PageDown => self.page(1),
            Key::Home => self.move_cursor(-((self.cursor % BYTES_PER_ROW) as i64)),
            Key::End => self.move_cursor((BYTES_PER_ROW - 1 - self.cursor % BYTES_PER_ROW) as i64),
            Key::G => self.goto_input = Some(String::new()),
            _ => {
                if let Some(d) = hex_digit(key) {
                    if self.editing {
                        self.edit(d, mem);
                    } else {
                        self.message = Some(("VIEW MODE - PRESS ENTER TO EDIT".into(), false));
                    }
                }
            }
        }
        Action::Stay
    }

    fn draw(&mut self, c: &mut Canvas, mem: &Apple2Memory) {
        c.fill(0, 0, COLS, crate::overlay::ROWS, BG);

        // Title bar with Language Card state (it decides what $D000+ shows).
        c.fill(0, 0, COLS, 1, ACCENT);
        let end = c.text(1, 0, "APPLE II MEMORY MONITOR", INVERSE_FG, ACCENT);
        if self.editing {
            c.text(end + 2, 0, " EDIT ", HILITE, ERROR);
        } else {
            c.text(end + 2, 0, " VIEW ", INVERSE_FG, ACCENT);
        }
        let lc = format!(
            "LC READ:{} WRITE:{} BANK:{}",
            if mem.lc_read_enable { "RAM" } else { "ROM" },
            if mem.lc_write_enable { "ON " } else { "OFF" },
            if mem.lc_bank2 { 2 } else { 1 },
        );
        c.text(COLS - 1 - lc.len(), 0, &lc, INVERSE_FG, ACCENT);

        // Column header.
        c.text(0, 2, "ADDR", DIM, BG);
        for i in 0..16usize {
            c.text(hex_col(i), 2, &format!("{:02X}", i), DIM, BG);
            c.text(ASCII_COL + i, 2, &format!("{:X}", i), DIM, BG);
        }

        for r in 0..DATA_ROWS {
            let row_addr = self.top + r * BYTES_PER_ROW;
            let y = FIRST_DATA_ROW + r as usize;
            c.text(0, y, &format!("{:04X}:", row_addr), ACCENT, BG);
            for i in 0..16u32 {
                let addr = row_addr + i;
                let v = mem.peek(addr as u16);
                let hex = v.map_or_else(|| "--".to_string(), |b| format!("{:02X}", b));
                let ascii = match v {
                    Some(b) if (0x20..0x60).contains(&(b & 0x7F)) => b & 0x7F,
                    _ => b'.',
                };
                let base_fg = if v.is_some() { TEXT } else { DIM };
                let x = hex_col(i as usize);
                let hb = hex.as_bytes();
                if addr == self.cursor && !self.editing {
                    c.text(x, y, &hex, INVERSE_FG, TEXT);
                    c.put(ASCII_COL + i as usize, y, ascii, INVERSE_FG, TEXT);
                } else if addr == self.cursor {
                    // Highlight the whole byte, or only the pending low nibble.
                    let (a, b) = if self.low_nibble { (false, true) } else { (true, true) };
                    c.put(x, y, hb[0], if a { INVERSE_FG } else { HILITE }, if a { HILITE } else { BG });
                    c.put(x + 1, y, hb[1], if b { INVERSE_FG } else { HILITE }, if b { HILITE } else { BG });
                    c.put(ASCII_COL + i as usize, y, ascii, INVERSE_FG, HILITE);
                } else {
                    c.text(x, y, &hex, base_fg, BG);
                    c.put(ASCII_COL + i as usize, y, ascii, base_fg, BG);
                }
            }
        }

        // Status line: goto prompt, message, or cursor info.
        if let Some(input) = &self.goto_input {
            let end = c.text(0, STATUS_ROW, "GOTO ADDRESS: $", HILITE, BG);
            let end = c.text(end, STATUS_ROW, input, TEXT, BG);
            c.put(end, STATUS_ROW, b' ', INVERSE_FG, HILITE);
            c.text(end + 3, STATUS_ROW, "(ENTER OK, ESC CANCEL)", DIM, BG);
        } else if let Some((msg, is_err)) = &self.message {
            c.text(0, STATUS_ROW, msg, if *is_err { ERROR } else { HILITE }, BG);
        } else {
            let v = mem.peek(self.cursor as u16);
            let info = match v {
                Some(b) => format!("${:04X} = ${:02X}  {:3}  %{:08b}", self.cursor, b, b, b),
                None => format!("${:04X} = I/O (NO STATIC VALUE)", self.cursor),
            };
            c.text(0, STATUS_ROW, &info, TEXT, BG);
        }

        c.fill(0, HELP_ROW, COLS, 1, ACCENT);
        c.text(
            1,
            HELP_ROW,
            if self.editing {
                "EDIT: 0-F WRITE BYTE  ARROWS MOVE  ENTER/ESC BACK TO VIEW"
            } else {
                "ARROWS MOVE  PGUP/PGDN PAGE  G GOTO  ENTER EDIT  ESC/F6 RESUME"
            },
            INVERSE_FG,
            ACCENT,
        );
    }
}

fn hex_col(i: usize) -> usize {
    HEX_COL + i * 3 + if i >= 8 { 1 } else { 0 }
}

fn hex_digit(key: Key) -> Option<u8> {
    Some(match key {
        Key::Key0 | Key::NumPad0 => 0,
        Key::Key1 | Key::NumPad1 => 1,
        Key::Key2 | Key::NumPad2 => 2,
        Key::Key3 | Key::NumPad3 => 3,
        Key::Key4 | Key::NumPad4 => 4,
        Key::Key5 | Key::NumPad5 => 5,
        Key::Key6 | Key::NumPad6 => 6,
        Key::Key7 | Key::NumPad7 => 7,
        Key::Key8 | Key::NumPad8 => 8,
        Key::Key9 | Key::NumPad9 => 9,
        Key::A => 10,
        Key::B => 11,
        Key::C => 12,
        Key::D => 13,
        Key::E => 14,
        Key::F => 15,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(v: &mut MemView, mem: &mut Apple2Memory, ks: &[Key]) {
        for &k in ks {
            v.key(k, false, mem);
        }
    }

    #[test]
    fn typing_hex_edits_and_advances() {
        let mut mem = Apple2Memory::new();
        let mut v = MemView::new(0x300);
        keys(&mut v, &mut mem, &[Key::Enter, Key::A, Key::Key9, Key::Key0, Key::Key0]);
        assert_eq!(&mem.ram[0x300..0x302], &[0xA9, 0x00]);
        assert_eq!(v.cursor, 0x302);
    }

    #[test]
    fn moving_resets_half_typed_byte() {
        let mut mem = Apple2Memory::new();
        mem.ram[0x300] = 0x12;
        let mut v = MemView::new(0x300);
        keys(&mut v, &mut mem, &[Key::Enter, Key::F, Key::Right, Key::Left, Key::Key7]);
        assert_eq!(mem.ram[0x300], 0x72);
    }

    #[test]
    fn view_mode_is_default_and_read_only() {
        let mut mem = Apple2Memory::new();
        let mut v = MemView::new(0x300);
        keys(&mut v, &mut mem, &[Key::A, Key::Key9]);
        assert_eq!(mem.ram[0x300], 0x00);
        assert_eq!(v.cursor, 0x300);
        keys(&mut v, &mut mem, &[Key::Enter, Key::Key1, Key::Key2, Key::Enter, Key::Key3, Key::Key4]);
        assert_eq!(&mem.ram[0x300..0x302], &[0x12, 0x00]);
        assert!(!v.editing);
    }

    #[test]
    fn goto_and_scroll_bounds() {
        let mut mem = Apple2Memory::new();
        let mut v = MemView::new(0);
        keys(&mut v, &mut mem, &[Key::G, Key::F, Key::F, Key::F, Key::F, Key::Enter]);
        assert_eq!(v.cursor, 0xFFFF);
        assert!(v.top + PAGE == 0x1_0000);
        keys(&mut v, &mut mem, &[Key::Right, Key::Down]);
        assert_eq!(v.cursor, 0xFFFF);
        keys(&mut v, &mut mem, &[Key::G, Key::Escape]);
        assert!(v.goto_input.is_none());
        keys(&mut v, &mut mem, &[Key::PageUp]);
        assert_eq!(v.cursor, 0xFFFF - PAGE);
    }

    #[test]
    fn io_is_not_editable_and_closes_on_escape() {
        let mut mem = Apple2Memory::new();
        let mut v = MemView::new(0xC050);
        keys(&mut v, &mut mem, &[Key::Enter, Key::Key1]);
        assert!(v.message.as_ref().unwrap().1);
        assert!(mem.text_mode);
        assert!(matches!(v.key(Key::Escape, false, &mut mem), Action::Stay)); // leaves edit mode
        assert!(matches!(v.key(Key::Escape, false, &mut mem), Action::Close));
    }

    #[test]
    fn draws_rows_with_expected_layout() {
        let mut mem = Apple2Memory::new();
        mem.ram[0x400] = 0xC1;
        let mut v = MemView::new(0x400);
        let mut c = Canvas::new();
        v.draw(&mut c, &mem);
        let row = c.row_string(FIRST_DATA_ROW);
        assert!(row.starts_with("0400: C1 00 00 00 00 00 00 00  00"), "{row}");
        assert_eq!(&row[ASCII_COL..ASCII_COL + 2], "A.");
    }
}
