//! Console memory monitor (F6). Command syntax follows the Apple II `*`
//! monitor so it feels familiar:
//!
//!   300          show one byte
//!   300.3FF      dump a range
//!   (empty)      dump the next 8 bytes
//!   300:A9 00 60 store bytes starting at $300
//!   :EA EA       keep storing after the last stored byte
//!   ?            help
//!   Q / F6       resume emulation
//!
//! All access goes through `peek`/`poke`, so looking at memory never flips
//! soft switches. Writes to $D000-$FFFF go into the current Language Card bank.

use apple2_core::memory::Apple2Memory;
use std::io::{BufRead, Write};
use std::sync::mpsc::{channel, Receiver};

const HELP: &str = "\
  300          show byte at $0300
  300.3FF      dump $0300-$03FF
  <Enter>      dump next 8 bytes
  300:A9 00 60 store bytes at $0300..
  :EA EA       continue storing after the last byte
  Q or F6      resume emulation
  (-- = I/O location with no static value; $D000+ writes go to the current LC bank)";

pub enum Outcome {
    Continue,
    Quit,
}

pub struct Monitor {
    /// Next address an empty line dumps from.
    next_examine: u16,
    /// Next address a bare `:` stores to.
    next_store: u16,
}

impl Monitor {
    pub fn new() -> Self {
        Self { next_examine: 0, next_store: 0 }
    }

    pub fn enter(&self) {
        println!("
=== Apple II memory monitor (emulation paused). ? for help, Q (or F6/Esc in the window) to resume ===");
        prompt();
    }

    pub fn leave(&self) {
        println!("
=== Resuming emulation ===");
    }

    /// Runs one console line and prints its output. Returns Quit on `Q`.
    pub fn handle_line(&mut self, line: &str, mem: &mut Apple2Memory) -> Outcome {
        let mut out = Vec::new();
        let outcome = self.execute(line.trim(), mem, &mut out);
        for l in out {
            println!("{}", l);
        }
        if let Outcome::Continue = outcome {
            prompt();
        }
        outcome
    }

    pub fn execute(&mut self, cmd: &str, mem: &mut Apple2Memory, out: &mut Vec<String>) -> Outcome {
        let cmd = cmd.trim();
        if cmd.eq_ignore_ascii_case("q") {
            return Outcome::Quit;
        }
        if cmd == "?" {
            out.extend(HELP.lines().map(str::to_string));
            return Outcome::Continue;
        }
        if let Err(e) = self.execute_inner(cmd, mem, out) {
            out.push(format!("ERR: {}", e));
        }
        Outcome::Continue
    }

    fn execute_inner(&mut self, cmd: &str, mem: &mut Apple2Memory, out: &mut Vec<String>) -> Result<(), String> {
        if cmd.is_empty() {
            let start = self.next_examine;
            self.dump(mem, start, start.saturating_add(7), out);
            return Ok(());
        }

        if let Some((addr_part, data_part)) = cmd.split_once(':') {
            let mut addr = if addr_part.trim().is_empty() {
                self.next_store
            } else {
                parse_hex(addr_part.trim())?
            };
            let bytes = data_part
                .split_whitespace()
                .map(|t| {
                    let v = parse_hex(t)?;
                    u8::try_from(v).map_err(|_| format!("{} is not a byte", t))
                })
                .collect::<Result<Vec<u8>, String>>()?;
            if bytes.is_empty() {
                return Err("no bytes to store".into());
            }
            let touched_lc = (addr as usize + bytes.len() - 1) >= 0xD000;
            for b in bytes {
                if !mem.poke(addr, b) {
                    return Err(format!("${:04X} is I/O space, not writable from the monitor", addr));
                }
                addr = addr.wrapping_add(1);
            }
            if touched_lc && !mem.lc_read_enable {
                out.push("note: LC RAM is not read-enabled, so $D000+ still shows ROM".into());
            }
            self.next_store = addr;
            return Ok(());
        }

        if let Some((a, b)) = cmd.split_once('.') {
            let start = parse_hex(a.trim())?;
            let end = parse_hex(b.trim())?;
            if end < start {
                return Err("range end is before start".into());
            }
            self.dump(mem, start, end, out);
            return Ok(());
        }

        let addr = parse_hex(cmd)?;
        out.push(format!("{:04X}- {}", addr, fmt_byte(mem.peek(addr))));
        self.next_examine = addr.wrapping_add(1);
        Ok(())
    }

    /// Dumps `start..=end`, 8 bytes per line aligned to 8, with ASCII column.
    fn dump(&mut self, mem: &Apple2Memory, start: u16, end: u16, out: &mut Vec<String>) {
        let mut line_start = start & !7;
        loop {
            let mut hex = String::new();
            let mut ascii = String::new();
            for i in 0..8u16 {
                let a = line_start.wrapping_add(i);
                if a < start || a > end {
                    hex.push_str("   ");
                    ascii.push(' ');
                } else {
                    let v = mem.peek(a);
                    hex.push_str(&format!(" {}", fmt_byte(v)));
                    ascii.push(match v {
                        Some(b) if (0x20..0x7F).contains(&(b & 0x7F)) => (b & 0x7F) as char,
                        _ => '.',
                    });
                }
            }
            out.push(format!("{:04X}-{}  {}", line_start.max(start), hex, ascii));
            match line_start.checked_add(8) {
                Some(n) if n <= end => line_start = n,
                _ => break,
            }
        }
        self.next_examine = end.wrapping_add(1);
    }
}

fn prompt() {
    print!("*");
    let _ = std::io::stdout().flush();
}

/// Reads stdin on a background thread so the main loop never blocks on it
/// and can keep pumping window events (e.g. F6 to resume) while paused.
pub fn spawn_stdin_reader() -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    rx
}

fn fmt_byte(v: Option<u8>) -> String {
    v.map_or_else(|| "--".to_string(), |b| format!("{:02X}", b))
}

fn parse_hex(s: &str) -> Result<u16, String> {
    let t = s.trim_start_matches('$');
    if t.is_empty() || t.len() > 4 {
        return Err(format!("bad hex '{}'", s));
    }
    u16::from_str_radix(t, 16).map_err(|_| format!("bad hex '{}'", s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(m: &mut Monitor, mem: &mut Apple2Memory, cmd: &str) -> Vec<String> {
        let mut out = Vec::new();
        m.execute(cmd, mem, &mut out);
        out
    }

    #[test]
    fn store_then_examine() {
        let mut m = Monitor::new();
        let mut mem = Apple2Memory::new();
        assert!(run(&mut m, &mut mem, "300:A9 00 60").is_empty());
        assert_eq!(&mem.ram[0x300..0x303], &[0xA9, 0x00, 0x60]);
        run(&mut m, &mut mem, ":EA");
        assert_eq!(mem.ram[0x303], 0xEA);
        assert_eq!(run(&mut m, &mut mem, "301"), vec!["0301- 00"]);
    }

    #[test]
    fn range_dump_and_continue() {
        let mut m = Monitor::new();
        let mut mem = Apple2Memory::new();
        mem.ram[0x400] = 0xC1; // 'A' with high bit
        let out = run(&mut m, &mut mem, "400.40F");
        assert_eq!(out.len(), 2);
        assert!(out[0].starts_with("0400- C1 00"));
        assert!(out[0].ends_with("A......."));
        let next = run(&mut m, &mut mem, "");
        assert!(next[0].starts_with("0410-"));
    }

    #[test]
    fn io_is_protected() {
        let mut m = Monitor::new();
        let mut mem = Apple2Memory::new();
        assert_eq!(run(&mut m, &mut mem, "C050"), vec!["C050- --"]);
        assert!(run(&mut m, &mut mem, "C050:00")[0].starts_with("ERR"));
        assert!(mem.text_mode);
    }

    #[test]
    fn bad_input_reports_error() {
        let mut m = Monitor::new();
        let mut mem = Apple2Memory::new();
        assert!(run(&mut m, &mut mem, "XYZ")[0].starts_with("ERR"));
        assert!(run(&mut m, &mut mem, "300:1FF")[0].starts_with("ERR"));
        assert!(matches!(m.execute("q", &mut mem, &mut Vec::new()), Outcome::Quit));
    }
}
