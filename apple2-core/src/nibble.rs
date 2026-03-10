extern crate alloc;
use alloc::vec::Vec;

// Apple II DOS 3.3 Nibble Conversion
// on the disk surface to avoid having too many consecutive zeroes.
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

// DOS 3.3 Logical to Physical Sector Interleaving mapping
pub const DOS33_SECTOR_MAP: [usize; 16] = [
    0, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 15
];

pub struct TrackData {
    pub raw_bytes: [u8; 6656], // A track is generally max ~6656 nibbles (16 sectors)
    pub length: usize,
}

impl TrackData {
    pub const fn new() -> Self {
        Self {
            raw_bytes: [0; 6656],
            length: 0,
        }
    }

    pub fn push(&mut self, val: u8) {
        if self.length < self.raw_bytes.len() {
            self.raw_bytes[self.length] = val;
            self.length += 1;
        }
    }
}

/// Convert a 140KB `.dsk` slice into a Vector of 35 raw nibblized tracks
pub fn nibblize_dsk(disk_data: &[u8]) -> alloc::vec::Vec<TrackData> {
    let mut tracks = alloc::vec::Vec::with_capacity(35);

    // DOS 3.3 physical sector interleave table
    // physical_position -> logical sector number in the .dsk file
    // e.g. the first sector on the track (physical 0) holds logical sector 0,
    //      the second sector on the track (physical 1) holds logical sector 7, etc.
    const PHYS_TO_LOGICAL: [usize; 16] = [0, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 15];

    for track_num in 0..35 {
        let mut track_out = TrackData::new();
        let track_offset = track_num * 16 * 256;

        // Gap 1 (Lead-in)
        for _ in 0..48 { track_out.push(0xFF); }

        for phys_pos in 0..16 {
            let logical_sector = PHYS_TO_LOGICAL[phys_pos];
            let sector_offset = track_offset + (logical_sector * 256);
            let sector_data = &disk_data[sector_offset .. sector_offset + 256];

            // Address Field
            // Prologue: D5 AA 96
            track_out.push(0xD5);
            track_out.push(0xAA);
            track_out.push(0x96);

            // Volume (default 254), Track, Sector, Checksum
            // The address field sector number is the LOGICAL sector number
            let vol = 254_u8;
            let trk = track_num as u8;
            let sec = logical_sector as u8;
            let chk = vol ^ trk ^ sec;

            // 4x4 encoding for address fields
            let encode4x4 = |val: u8| -> (u8, u8) {
                ((val >> 1) | 0xAA, val | 0xAA)
            };

            let (v1, v2) = encode4x4(vol);
            track_out.push(v1); track_out.push(v2);

            let (t1, t2) = encode4x4(trk);
            track_out.push(t1); track_out.push(t2);

            let (s1, s2) = encode4x4(sec);
            track_out.push(s1); track_out.push(s2);

            let (c1, c2) = encode4x4(chk);
            track_out.push(c1); track_out.push(c2);

            // Epilogue: DE AA EB
            track_out.push(0xDE);
            track_out.push(0xAA);
            track_out.push(0xEB);

            // Gap 2
            for _ in 0..10 { track_out.push(0xFF); }

            // Data Field
            // Prologue: D5 AA AD
            track_out.push(0xD5);
            track_out.push(0xAA);
            track_out.push(0xAD);

            // 6-and-2 Data Encoding (standard algorithm from "Beneath Apple DOS")
            // Step 1: Build the 342 pre-nibblized bytes
            let mut nbuf2 = [0u8; 342]; // 0..85 = secondary, 86..341 = primary
            
            // Primary buffer: top 6 bits of each data byte
            for i in 0..256 {
                nbuf2[86 + i] = sector_data[i] >> 2;
            }
            
            // Secondary buffer: bottom 2 bits, packed 3 per byte
            // The standard algorithm processes data bytes 0 up to 255,
            // shifting the previous bits left by 2 so that larger indices end up at the bottom
            for i in 0..256 {
                let val = sector_data[i];
                // Extract bottom 2 bits, bit-reversed (matches hardware LSR/ROL order)
                let two_bits = ((val & 0x01) << 1) | ((val & 0x02) >> 1);
                let slot = i % 86;
                nbuf2[slot] = (nbuf2[slot] << 2) | two_bits;
            }

            // Write out nbuf2 mapped through NIBBLE_WRITE_TABLE with checksums
            let mut last = 0;
            for i in 0..342 {
                let val = nbuf2[i];
                track_out.push(NIBBLE_WRITE_TABLE[(val ^ last) as usize]);
                last = val;
            }
            track_out.push(NIBBLE_WRITE_TABLE[last as usize]);

            // Epilogue: DE AA EB
            track_out.push(0xDE);
            track_out.push(0xAA);
            track_out.push(0xEB);

            // Gap 3
            for _ in 0..40 { track_out.push(0xFF); }
        }

        tracks.push(track_out);
    }

    tracks
}
