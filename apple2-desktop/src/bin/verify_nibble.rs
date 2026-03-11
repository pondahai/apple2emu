/// Verify nibble encoding/decoding round-trip for DOS 3.3
/// Run: cargo run --bin verify_nibble
use apple2_core::nibble::{nibblize_dsk, NIBBLE_WRITE_TABLE};

fn main() {
    let roms_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .map(|p| p.join("roms"))
        .unwrap_or_else(|| std::path::PathBuf::from("../roms"));

    let dsk_path = roms_dir.join("MASTER.DSK");
    let disk_data = std::fs::read(&dsk_path).expect("Cannot read MASTER.DSK");

    println!("=== Raw .dsk sector 0 first 16 bytes ===");
    for b in &disk_data[..16] { print!("{:02X} ", b); }
    println!();
    println!("Lower 2 bits of each byte:");
    for b in &disk_data[..16] { print!("{:02b} ", b & 3); }
    println!();

    // Build reverse lookup table: GCR byte -> 6-bit value
    let mut gcr_decode = [0xFFu8; 256];
    for (i, &gcr) in NIBBLE_WRITE_TABLE.iter().enumerate() {
        gcr_decode[gcr as usize] = i as u8;
    }

    let tracks = nibblize_dsk(&disk_data);
    let track0 = &tracks[0];
    let raw = &track0.raw_bytes[..track0.length];

    // Find sector 0 data field
    let mut i = 0;
    while i + 20 < raw.len() {
        if raw[i] == 0xD5 && raw[i+1] == 0xAA && raw[i+2] == 0x96 {
            let decode4x4 = |hi: u8, lo: u8| -> u8 { ((hi & 0x55) << 1) | (lo & 0x55) };
            if i + 11 < raw.len() {
                let sec = decode4x4(raw[i+7], raw[i+8]);
                if sec == 0 {
                    println!("\n=== Sector 0 found at idx={} ===", i);
                    // Find data field
                    let mut j = i + 14;
                    while j < raw.len().min(i + 40) {
                        if raw[j] == 0xD5 && raw[j+1] == 0xAA && raw[j+2] == 0xAD {
                            let data_start = j + 3;
                            // Decode with XOR chain - same as Apple II RWTS
                            let mut nbuf = [0u8; 342];
                            let mut last = 0u8;
                            for k in 0..342 {
                                let gcr_byte = raw[data_start + k];
                                let val6 = gcr_decode[gcr_byte as usize];
                                let decoded = val6 ^ last;
                                nbuf[k] = decoded;
                                last = decoded;
                            }
                            // Checksum byte
                            let chk_gcr = raw[data_start + 342];
                            let chk_val = gcr_decode[chk_gcr as usize] ^ last;
                            println!("  Checksum nibble (should be 0): {:02X}", chk_val);

                            println!("\n  Secondary buffer (nbuf[0..16]):");
                            for k in 0..16 { print!("  {:06b}", nbuf[k]); }
                            println!();

                            println!("\n  Expected secondary nibbles for sector[0..16]:");
                            for k in 0..16usize {
                                // According to our encoding:
                                // slot k bits[1:0] = sector[k].bit1, sector[k].bit0
                                let expected = (disk_data[k] & 0x03) | 
                                    (if k + 86 < 256 { ((disk_data[k+86] & 0x03) << 2) } else { 0 }) |
                                    (if k + 172 < 256 { ((disk_data[k+172] & 0x03) << 4) } else { 0 });
                                print!("  {:06b}", expected);
                            }
                            println!();

                            // Try both possible secondary bit arrangements
                            println!("\n=== Decode attempt A: bits[1:0]->sector[i], bits[3:2]->sector[i+86], bits[5:4]->sector[i+172] ===");
                            let mut sector_a = [0u8; 256];
                            for k in 0..256usize { sector_a[k] = nbuf[86 + k] << 2; }
                            for k in 0..86usize {
                                let s = nbuf[k];
                                sector_a[k]          |= s & 0x03;
                                if k + 86 < 256  { sector_a[k + 86]  |= (s >> 2) & 0x03; }
                                if k + 172 < 256 { sector_a[k + 172] |= (s >> 4) & 0x03; }
                            }
                            print!("  First 8: "); for b in &sector_a[..8] { print!("{:02X} ", b); } println!();
                            print!("  Expected: "); for b in &disk_data[..8] { print!("{:02X} ", b); } 
                            let match_a = sector_a[..8] == disk_data[..8];
                            println!("  {}", if match_a { "✓ MATCH" } else { "✗ MISMATCH" });

                            println!("\n=== Decode attempt B: bits[5:4]->sector[i], bits[3:2]->sector[i+86], bits[1:0]->sector[i+172] (reversed) ===");
                            let mut sector_b = [0u8; 256];
                            for k in 0..256usize { sector_b[k] = nbuf[86 + k] << 2; }
                            for k in 0..86usize {
                                let s = nbuf[k];
                                sector_b[k]          |= (s >> 4) & 0x03;  // bits 5:4
                                if k + 86 < 256  { sector_b[k + 86]  |= (s >> 2) & 0x03; }
                                if k + 172 < 256 { sector_b[k + 172] |= s & 0x03; }
                            }
                            print!("  First 8: "); for b in &sector_b[..8] { print!("{:02X} ", b); } println!();
                            print!("  Expected: "); for b in &disk_data[..8] { print!("{:02X} ", b); }
                            let match_b = sector_b[..8] == disk_data[..8];
                            println!("  {}", if match_b { "✓ MATCH" } else { "✗ MISMATCH" });

                            println!("\n=== Decode attempt C: RWTS actual bit reversal ===");
                            // Apple II RWTS denibble: for each slot, LSR twice into carry, ROL into buf
                            // This means bit0 of nibble -> bit0 of sector[i], bit1 -> bit1
                            // But due to ROL sequence: sec[i] gets bits via ROL (left shift + carry)
                            // Let's try: bits reversed within each pair
                            let mut sector_c = [0u8; 256];
                            for k in 0..256usize { sector_c[k] = nbuf[86 + k] << 2; }
                            for k in 0..86usize {
                                let s = nbuf[k];
                                // Reverse bits within each 2-bit pair
                                let pair0 = ((s & 0x01) << 1) | ((s >> 1) & 0x01);  // swap bits 1:0
                                let pair1 = ((s & 0x04) >> 1) | ((s >> 3) & 0x01);  // swap bits 3:2  
                                let pair2 = ((s & 0x10) >> 3) | ((s >> 5) & 0x01);  // swap bits 5:4
                                sector_c[k]          |= pair0;
                                if k + 86 < 256  { sector_c[k + 86]  |= pair1; }
                                if k + 172 < 256 { sector_c[k + 172] |= pair2; }
                            }
                            print!("  First 8: "); for b in &sector_c[..8] { print!("{:02X} ", b); } println!();
                            print!("  Expected: "); for b in &disk_data[..8] { print!("{:02X} ", b); }
                            let match_c = sector_c[..8] == disk_data[..8];
                            println!("  {}", if match_c { "✓ MATCH" } else { "✗ MISMATCH" });
                            break;
                        }
                        j += 1;
                    }
                    break;
                }
            }
        }
        i += 1;
    }
}
