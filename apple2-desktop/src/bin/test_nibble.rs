use apple2_core::nibble::{nibblize_dsk, denibblize_dsk};

fn main() {
    let disk_path = r"C:\Users\Dell\Downloads\AppleWin1.30.18.0\MASTER.DSK";
    let disk = std::fs::read(disk_path).expect("Failed to read MASTER.DSK");
    
    let tracks = nibblize_dsk(&disk);
    
    let denibblized = denibblize_dsk(&tracks).expect("Failed to denibblize");
    
    let mut failed_sectors = vec![];
    for t in 0..35 {
        for s in 0..16 {
            let offset = t * 16 * 256 + s * 256;
            let mut mismatch = false;
            for i in 0..256 {
                if disk[offset + i] != denibblized[offset + i] {
                    mismatch = true;
                    break;
                }
            }
            if mismatch {
                failed_sectors.push((t, s));
            }
        }
    }
    
    if failed_sectors.is_empty() {
        println!("SUCCESS: denibblized data perfectly matches original disk data!");
    } else {
        println!("FAILURE: {} sectors mismatched", failed_sectors.len());
        println!("First 10 failed sectors: {:?}", &failed_sectors[..failed_sectors.len().min(10)]);
    }
    
    // Check track 0 specifically
    for i in 0..16 {
        let expected = disk[i * 256];
        let actual = denibblized[i * 256];
        println!("Track 0 Sector {}: expected {:02X}, got {:02X}", i, expected, actual);
    }
}

