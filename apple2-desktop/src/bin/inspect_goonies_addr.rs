use std::io::Read;
use std::path::PathBuf;

use apple2_core::nibble::nibblize_dsk;
use flate2::read::GzDecoder;

fn decode_disk_image(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let raw_data =
        std::fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let is_gz_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gz"))
        .unwrap_or(false);
    let is_gz_magic = raw_data.len() >= 2 && raw_data[0] == 0x1F && raw_data[1] == 0x8B;

    let disk = if is_gz_ext || is_gz_magic {
        let mut decoder = GzDecoder::new(raw_data.as_slice());
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| format!("Failed to decompress {}: {}", path.display(), e))?;
        out
    } else {
        raw_data
    };

    if disk.len() != 143_360 {
        return Err(format!(
            "Disk size mismatch for {}: {} (expected 143360 bytes)",
            path.display(),
            disk.len()
        ));
    }

    Ok(disk)
}

fn decode_4x4(hi: u8, lo: u8) -> u8 {
    ((hi << 1) | 1) & lo
}

fn main() {
    let disk_path =
        PathBuf::from(r"C:\Users\pondahai\Downloads\AppleWin1.26.1.1\ac\goonies.dsk.gz");
    let disk = decode_disk_image(&disk_path).expect("decode goonies");
    let tracks = nibblize_dsk(&disk);
    let track = &tracks[23];

    println!("track=23 len={}", track.length);

    let mut found = 0usize;
    for i in 0..track.length {
        if track.raw_bytes[i] == 0xD5
            && track.raw_bytes[(i + 1) % track.length] == 0xAA
            && track.raw_bytes[(i + 2) % track.length] == 0x96
        {
            let vol = decode_4x4(track.raw_bytes[(i + 3) % track.length], track.raw_bytes[(i + 4) % track.length]);
            let trk = decode_4x4(track.raw_bytes[(i + 5) % track.length], track.raw_bytes[(i + 6) % track.length]);
            let sec = decode_4x4(track.raw_bytes[(i + 7) % track.length], track.raw_bytes[(i + 8) % track.length]);
            let chk = decode_4x4(track.raw_bytes[(i + 9) % track.length], track.raw_bytes[(i + 10) % track.length]);
            println!(
                "addr idx={:04} vol={:02X} trk={:02X} sec={:02X} chk={:02X} expect_chk={:02X}",
                i,
                vol,
                trk,
                sec,
                chk,
                vol ^ trk ^ sec
            );
            found += 1;
        }
    }

    println!("address fields listed={}", found);
}
