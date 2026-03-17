extern crate alloc;

pub const NIBBLE_WRITE_TABLE: [u8; 64] = [
    0x96, 0x97, 0x9A, 0x9B, 0x9D, 0x9E, 0x9F, 0xA6, 0xA7, 0xAB, 0xAC, 0xAD, 0xAE, 0xAF, 0xB2, 0xB3,
    0xB4, 0xB5, 0xB6, 0xB7, 0xB9, 0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xBF, 0xCB, 0xCD, 0xCE, 0xCF, 0xD3,
    0xD6, 0xD7, 0xD9, 0xDA, 0xDB, 0xDC, 0xDD, 0xDE, 0xDF, 0xE5, 0xE6, 0xE7, 0xE9, 0xEA, 0xEB, 0xEC,
    0xED, 0xEE, 0xEF, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF,
];

// DOS 3.3 physical-to-logical sector interleave table
pub const PHYS_TO_LOGICAL: [usize; 16] = [0, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 15];

pub struct TrackData {
    pub raw_bytes: [u8; 6656],
    pub length: usize,
    pub read_length: usize,
}

impl TrackData {
    pub const fn new() -> Self {
        Self {
            raw_bytes: [0; 6656],
            length: 0,
            read_length: 0,
        }
    }
    pub fn push(&mut self, val: u8) {
        if self.length < self.raw_bytes.len() {
            self.raw_bytes[self.length] = val;
            self.length += 1;
        }
    }
}

pub fn nibblize_dsk(disk_data: &[u8]) -> alloc::vec::Vec<TrackData> {
    let mut tracks = alloc::vec::Vec::with_capacity(35);
    for track_num in 0..35 {
        let mut track_out = TrackData::new();
        let track_offset = track_num * 16 * 256;

        // Pre-gap: 64 self-sync bytes
        for _ in 0..64 {
            track_out.push(0xFF);
        }

        for phys_pos in 0..16 {
            let logical_sector = PHYS_TO_LOGICAL[phys_pos];
            let sector_data = &disk_data
                [track_offset + logical_sector * 256..track_offset + logical_sector * 256 + 256];

            // Address Field
            track_out.push(0xD5);
            track_out.push(0xAA);
            track_out.push(0x96);

            let vol: u8 = 254;
            let trk: u8 = track_num as u8;
            // sec = physical sector number (what RWTS searches for)
            let sec: u8 = phys_pos as u8;
            let chk: u8 = vol ^ trk ^ sec;

            let encode4x4 = |val: u8| -> (u8, u8) { ((val >> 1) | 0xAA, val | 0xAA) };
            let (v1, v2) = encode4x4(vol);
            track_out.push(v1);
            track_out.push(v2);
            let (t1, t2) = encode4x4(trk);
            track_out.push(t1);
            track_out.push(t2);
            let (s1, s2) = encode4x4(sec);
            track_out.push(s1);
            track_out.push(s2);
            let (c1, c2) = encode4x4(chk);
            track_out.push(c1);
            track_out.push(c2);

            track_out.push(0xDE);
            track_out.push(0xAA);
            track_out.push(0xEB);

            // Inter-field gap: 6 self-sync bytes
            for _ in 0..6 {
                track_out.push(0xFF);
            }

            // Data Field
            track_out.push(0xD5);
            track_out.push(0xAA);
            track_out.push(0xAD);

            // 6-and-2 encoding per "Beneath Apple DOS" Chapter 3
            //
            // For each index k in 0..86, the secondary nibble snib[k] is:
            //   bits [1:0] = sector_data[k]       & 0x03  (group 0, offset 0)
            //   bits [3:2] = sector_data[k + 86]  & 0x03  (group 1, offset 86)
            //   bits [5:4] = sector_data[k + 172] & 0x03  (group 2, offset 172, k<84 only)
            //
            // No bit-swap. The secondary buffer is emitted REVERSED (snib[85]..snib[0]).
            let swap2 = |b: u8| -> u8 { ((b & 0x01) << 1) | ((b & 0x02) >> 1) };

            let mut snib = [0u8; 86];
            for k in 0..86 {
                let b0 = swap2(sector_data[k] & 0x03);
                let b1 = swap2(sector_data[k + 86] & 0x03);
                let b2 = if k + 172 < 256 {
                    swap2(sector_data[k + 172] & 0x03)
                } else {
                    0
                };
                snib[k] = (b2 << 4) | (b1 << 2) | b0;
            }

            let mut last_val: u8 = 0;

            for i in 0..86 {
                let val6 = snib[i] & 0x3F;
                track_out.push(NIBBLE_WRITE_TABLE[(val6 ^ last_val) as usize]);
                last_val = val6;
            }

            for i in 0..256 {
                let val6 = sector_data[i] >> 2;
                track_out.push(NIBBLE_WRITE_TABLE[(val6 ^ last_val) as usize]);
                last_val = val6;
            }

            // final checksum nibble
            track_out.push(NIBBLE_WRITE_TABLE[last_val as usize]);

            track_out.push(0xDE);
            track_out.push(0xAA);
            track_out.push(0xEB);

            // Inter-sector gap: 27 self-sync bytes
            for _ in 0..27 {
                track_out.push(0xFF);
            }
        }
        track_out.read_length = track_out.length;
        tracks.push(track_out);
    }
    tracks
}

pub fn denibblize_dsk(tracks: &[TrackData]) -> Result<alloc::vec::Vec<u8>, alloc::string::String> {
    let mut out = alloc::vec![0u8; 35 * 16 * 256];
    
    let mut read_table = [0xFFu8; 256];
    for (i, &val) in NIBBLE_WRITE_TABLE.iter().enumerate() {
        read_table[val as usize] = i as u8;
    }

    let decode4x4 = |v1: u8, v2: u8| -> u8 {
        ((v1 & 0x55) << 1) | (v2 & 0x55)
    };

    let swap2 = |b: u8| -> u8 { ((b & 0x01) << 1) | ((b & 0x02) >> 1) };

    for (track_num, track) in tracks.iter().enumerate() {
        if track.length == 0 {
            continue;
        }
        let data = &track.raw_bytes[..track.length];
        
        let mut sectors_found = [false; 16];
        let mut scan_idx = 0;
        let max_scan = track.length * 2;
        
        while scan_idx < max_scan {
            let i_mod = scan_idx % track.length;
            let i1_mod = (scan_idx + 1) % track.length;
            let i2_mod = (scan_idx + 2) % track.length;
            
            if data[i_mod] == 0xD5 && data[i1_mod] == 0xAA && data[i2_mod] == 0x96 {
                let get_byte = |offset: usize| data[(scan_idx + offset) % track.length];
                
                let vol = decode4x4(get_byte(3), get_byte(4));
                let trk = decode4x4(get_byte(5), get_byte(6));
                let sec = decode4x4(get_byte(7), get_byte(8));
                let chk = decode4x4(get_byte(9), get_byte(10));
                
                if (vol ^ trk ^ sec) != chk || sec >= 16 {
                    if track_num == 0 {
                        std::println!("Header fail: vol={}, trk={}, sec={}, chk={}", vol, trk, sec, chk);
                    }
                    scan_idx += 1;
                    continue;
                }
                
                if sectors_found[sec as usize] {
                    scan_idx += 14;
                    continue;
                }
                
                let mut data_idx = scan_idx + 14;
                let mut found_data = false;
                for j in data_idx..data_idx + 60 {
                    let j0 = j % track.length;
                    let j1 = (j + 1) % track.length;
                    let j2 = (j + 2) % track.length;
                    if data[j0] == 0xD5 && data[j1] == 0xAA && data[j2] == 0xAD {
                        found_data = true;
                        data_idx = j + 3;
                        break;
                    }
                }
                
                if !found_data {
                    if track_num == 0 {
                        std::println!("No data prologue for sec {}", sec);
                    }
                    scan_idx += 14;
                    continue;
                }
                
                let mut decoded = [0u8; 342];
                let mut last_val = 0u8;
                let mut read_err = false;
                
                for k in 0..342 {
                    let disk_val = data[(data_idx + k) % track.length];
                    let val6_from_disk = read_table[disk_val as usize];
                    if val6_from_disk == 0xFF {
                        if track_num == 0 {
                            std::println!("Invalid nibble for sec {} at byte {}", sec, k);
                        }
                        read_err = true;
                        break;
                    }
                    decoded[k] = val6_from_disk ^ last_val;
                    last_val = decoded[k];
                }
                
                let checksum_disk = data[(data_idx + 342) % track.length];
                let checksum_val6 = read_table[checksum_disk as usize];
                if checksum_val6 == 0xFF || checksum_val6 != last_val {
                    if track_num == 0 {
                        std::println!("Checksum fail for sec {}: read {:02X}, expected {:02X}", sec, checksum_val6, last_val);
                    }
                    read_err = true;
                }
                
                if read_err {
                    scan_idx = data_idx;
                    continue;
                }
                
                let mut sector_data = [0u8; 256];
                let snib = &decoded[0..86];
                let pbuf = &decoded[86..342]; 
                
                for k in 0..256 {
                    let primary = pbuf[k];
                    let sec_idx = k % 86;
                    let group = k / 86; 
                    
                    let b = match group {
                        0 => snib[sec_idx] & 0x03,
                        1 => (snib[sec_idx] >> 2) & 0x03,
                        2 => (snib[sec_idx] >> 4) & 0x03,
                        _ => 0,
                    };
                    
                    let orig_b = swap2(b);
                    sector_data[k] = (primary << 2) | orig_b;
                }
                
                let logical_sector = PHYS_TO_LOGICAL[sec as usize];
                let track_offset = track_num * 16 * 256;
                let sector_offset = track_offset + logical_sector * 256;
                out[sector_offset..sector_offset + 256].copy_from_slice(&sector_data);
                
                sectors_found[sec as usize] = true;
                scan_idx = data_idx + 343;
                
                let mut all_found = true;
                for b in &sectors_found {
                    if !*b {
                        all_found = false;
                        break;
                    }
                }
                if all_found {
                    break;
                }
            } else {
                scan_idx += 1;
            }
        }
    }
    
    Ok(out)
}
