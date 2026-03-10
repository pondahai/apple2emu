use apple2_core::nibble::nibblize_dsk;

fn main() {
    // Load the actual MASTER.DSK
    let disk = std::fs::read(r"C:\Users\Dell\Downloads\AppleWin1.30.18.0\MASTER.DSK").unwrap();

    let tracks = nibblize_dsk(&disk);
    let track0 = &tracks[0];
    
    // The boot ROM reads the sector with physical sector 0 (address field says sector=0)
    // In our nibblize, logical_sector 0 writes physical_sector = DOS33_SECTOR_MAP[0] = 0
    // So logical sector 0 = physical sector 0, meaning logical data offset 0..255
    
    // Check what bytes 0,2,86,88,172,174 are in the raw DSK:
    println!("DSK raw sector 0 data:");
    for i in [0usize, 1, 2, 3, 86, 87, 88, 89, 172, 173, 174, 175] {
        println!("  data[{}] = {:02X} (bottom 2 bits: {:02b})", i, disk[i], disk[i] & 3);
    }
    
    // Now simulate the RWTS postnibblize
    // Find the data for physical sector 0 (the first data field on the track)
    // The address field for logical sector 0 has physical_sector = 0
    
    // Find address prologues and match physical sector 0
    let mut data_offset = None;
    let mut i = 0;
    while i < track0.length - 20 {
        if track0.raw_bytes[i] == 0xD5 && 
           track0.raw_bytes[i+1] == 0xAA && 
           track0.raw_bytes[i+2] == 0x96 {
            // Address field: D5 AA 96 VV VV TT TT SS SS CC CC DE AA EB
            // Decode 4x4 sector
            let s1 = track0.raw_bytes[i+7];
            let s2 = track0.raw_bytes[i+8];
            let phys_sec = ((s1 << 1) | 1) & s2;
            
            if phys_sec == 0 {
                // Found physical sector 0! The data field follows after the address epilogue + gap
                // Search for data prologue D5 AA AD
                for j in (i+12)..(i+50).min(track0.length) {
                    if track0.raw_bytes[j] == 0xD5 && 
                       track0.raw_bytes[j+1] == 0xAA && 
                       track0.raw_bytes[j+2] == 0xAD {
                        data_offset = Some(j + 3);
                        break;
                    }
                }
                break;
            }
        }
        i += 1;
    }
    
    if let Some(off) = data_offset {
        println!("\nData field for phys sector 0 starts at offset {}", off);
        
        // Build reverse lookup
        let rev = {
            let mut r = [0u8; 256];
            for j in 0..64 { r[apple2_core::nibble::NIBBLE_WRITE_TABLE[j] as usize] = j as u8; }
            r
        };
        
        // Decode 343 nibble-encoded bytes
        let mut decoded = [0u8; 343];
        for j in 0..343 { decoded[j] = rev[track0.raw_bytes[off + j] as usize]; }
        
        // Un-XOR
        let mut nibbles = [0u8; 342];
        let mut prev = 0u8;
        for j in 0..342 { nibbles[j] = decoded[j] ^ prev; prev = nibbles[j]; }
        let cksum = decoded[342] ^ prev;
        println!("Checksum: {:02X}", cksum);
        
        // Simulate RWTS postnibblize (exact 6502 emulation)
        let mut sec = [0u8; 86];
        let mut pri = [0u8; 256];
        sec.copy_from_slice(&nibbles[0..86]);
        pri.copy_from_slice(&nibbles[86..342]);
        
        // Postnibblize: Y=0..255 ascending, X=85..0 descending wrapping
        let mut x: i32 = 85;
        for y in 0..256 {
            let s = sec[x as usize];
            // LSR sec: bit0 -> carry
            let c0 = s & 1;
            let s = s >> 1;
            // ROL pri: shift left, carry into bit0
            pri[y] = (pri[y] << 1) | c0;
            
            // LSR sec: bit0 -> carry
            let c1 = s & 1;
            let s = s >> 1;
            // ROL pri: shift left, carry into bit0
            pri[y] = (pri[y] << 1) | c1;
            
            // Store modified sec back
            sec[x as usize] = s;
            
            // DEX
            x -= 1;
            if x < 0 { x = 85; }
        }
        
        println!("\nReconstructed bytes 0..7:");
        for i in 0..8 {
            println!("  reconstructed[{}] = {:02X} (expected: {:02X}) {}", 
                     i, pri[i], disk[i],
                     if pri[i] == disk[i] { "OK" } else { "MISMATCH" });
        }
    }
}
