extern crate alloc;

pub const NIBBLE_WRITE_TABLE: [u8; 64] = [
    0x96, 0x97, 0x9A, 0x9B, 0x9D, 0x9E, 0x9F, 0xA6,
    0xA7, 0xAB, 0xAC, 0xAD, 0xAE, 0xAF, 0xB2, 0xB3,
    0xB4, 0xB5, 0xB6, 0xB7, 0xB9, 0xBA, 0xBB, 0xBC,
    0xBD, 0xBE, 0xBF, 0xCB, 0xCD, 0xCE, 0xCF, 0xD3,
    0xD6, 0xD7, 0xD9, 0xDA, 0xDB, 0xDC, 0xDD, 0xDE,
    0xDF, 0xE5, 0xE6, 0xE7, 0xE9, 0xEA, 0xEB, 0xEC,
    0xED, 0xEE, 0xEF, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6,
    0xF7, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF,
];

pub const PHYS_TO_LOGICAL: [usize; 16] = [0, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 15];

pub struct TrackData {
    pub raw_bytes: [u8; 6656],
    pub length: usize,
}

impl TrackData {
    pub const fn new() -> Self { Self { raw_bytes: [0; 6656], length: 0 } }
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
        for _ in 0..128 { track_out.push(0xFF); }
        for phys_pos in 0..16 {
            let logical_sector = PHYS_TO_LOGICAL[phys_pos];
            let sector_data = &disk_data[track_offset + (logical_sector * 256) .. track_offset + (logical_sector * 256) + 256];

            track_out.push(0xD5); track_out.push(0xAA); track_out.push(0x96);
            let vol = 254_u8; let trk = track_num as u8; let sec = phys_pos as u8; let chk = vol ^ trk ^ sec;
            let encode4x4 = |val: u8| -> (u8, u8) { ((val >> 1) | 0xAA, val | 0xAA) };
            let (v1, v2) = encode4x4(vol); track_out.push(v1); track_out.push(v2);
            let (t1, t2) = encode4x4(trk); track_out.push(t1); track_out.push(t2);
            let (s1, s2) = encode4x4(sec); track_out.push(s1); track_out.push(s2);
            let (c1, c2) = encode4x4(chk); track_out.push(c1); track_out.push(c2);
            track_out.push(0xDE); track_out.push(0xAA); track_out.push(0xEB);
            for _ in 0..20 { track_out.push(0xFF); }

            track_out.push(0xD5); track_out.push(0xAA); track_out.push(0xAD);
            
            let mut nbuf = [0u8; 342];
            let swap = |v: u8| -> u8 { ((v & 1) << 1) | ((v >> 1) & 1) };

            for i in 0..86 {
                let b0 = sector_data[i];
                let b1 = if i + 86 < 256 { sector_data[i + 86] } else { 0 };
                let b2 = if i + 172 < 256 { sector_data[i + 172] } else { 0 };
                
                let mut v = 0u8;
                v |= swap(b0 & 0x03) << 4;
                v |= swap(b1 & 0x03) << 2;
                v |= swap(b2 & 0x03) << 0;
                nbuf[i] = v;
            }
            for i in 0..256 { nbuf[86 + i] = sector_data[i] >> 2; }

            // P5 ROM Reversal logic
            let mut final_nbuf = [0u8; 342];
            for i in 0..86 { final_nbuf[i] = nbuf[85 - i]; }
            for i in 86..342 { final_nbuf[i] = nbuf[i]; }

            let mut last_val = 0u8;
            for i in 0..342 {
                let current_val = final_nbuf[i] & 0x3F;
                track_out.push(NIBBLE_WRITE_TABLE[(current_val ^ last_val) as usize]);
                last_val = current_val;
            }
            track_out.push(NIBBLE_WRITE_TABLE[last_val as usize]);
            track_out.push(0xDE); track_out.push(0xAA); track_out.push(0xEB);
            for _ in 0..40 { track_out.push(0xFF); }
        }
        tracks.push(track_out);
    }
    tracks
}
