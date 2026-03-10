use std::fs::File;
use std::io::{Read, Write};

fn main() {
    let ids = [134, 135, 136, 137, 138, 152];
    let mut out = File::create("preview_all.txt").unwrap();

    for id in ids {
        writeln!(out, "=== ROM {} ===", id).unwrap();
        if let Ok(mut f) = File::open(format!("../roms/extracted_2048_{}.bin", id)) {
            let mut buf = [0u8; 2048];
            if f.read_exact(&mut buf).is_ok() {
                // Just print 'A' at offset 0x41 or 0x01
                for c in [0x01, 0x41, 0xC1] {
                    writeln!(out, "Char {:02X}:", c).unwrap();
                    for row in 0..8 {
                        let b = buf[c * 8 + row];
                        let mut s = String::new();
                        for bit in 0..8 {
                            if b & (1 << bit) != 0 { s.push('#'); } else { s.push(' '); }
                        }
                        writeln!(out, "{}", s).unwrap();
                    }
                }
            }
        }
    }
}
