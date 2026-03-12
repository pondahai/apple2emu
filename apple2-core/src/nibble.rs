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

// DOS 3.3 physical-to-logical sector interleave table
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

        // Pre-gap: 64 self-sync bytes (0xFF)
        for _ in 0..64 { track_out.push(0xFF); }

        for phys_pos in 0..16 {
            let logical_sector = PHYS_TO_LOGICAL[phys_pos];
            let sector_data = &disk_data[
                track_offset + (logical_sector * 256)
                ..
                track_offset + (logical_sector * 256) + 256
            ];

            // ── Address Field ──────────────────────────────────────────
            track_out.push(0xD5); track_out.push(0xAA); track_out.push(0x96);

            let vol: u8 = 254;
            let trk: u8 = track_num as u8;
            // ★ FIX: sec must be the LOGICAL sector number, not phys_pos
            let sec: u8 = logical_sector as u8;
            let chk: u8 = vol ^ trk ^ sec;

            // 4-and-4 encoding: odd bits | 0xAA, even bits | 0xAA
            let encode4x4 = |val: u8| -> (u8, u8) {
                ((val >> 1) | 0xAA, val | 0xAA)
            };
            let (v1, v2) = encode4x4(vol); track_out.push(v1); track_out.push(v2);
            let (t1, t2) = encode4x4(trk); track_out.push(t1); track_out.push(t2);
            let (s1, s2) = encode4x4(sec); track_out.push(s1); track_out.push(s2);
            let (c1, c2) = encode4x4(chk); track_out.push(c1); track_out.push(c2);

            track_out.push(0xDE); track_out.push(0xAA); track_out.push(0xEB);

            // Inter-field gap: 6 self-sync bytes
            for _ in 0..6 { track_out.push(0xFF); }

            // ── Data Field ─────────────────────────────────────────────
            track_out.push(0xD5); track_out.push(0xAA); track_out.push(0xAD);

            // 6-and-2 encoding (Beneath Apple DOS, Chapter 3)
            // Step 1: build the 86-byte secondary nibble array (snib)
            //   Each byte contributes its low 2 bits to snib.
            //   The 256 bytes are split into 3 groups of 86 (last group has 84):
            //     group 0: bytes   0..85  → bits 1:0 of snib[85-k]
            //     group 1: bytes  86..171 → bits 3:2 of snib[85-(k-86)]
            //     group 2: bytes 172..255 → bits 5:4 of snib[83-(k-172)]
            //   The 2-bit value is also bit-reversed (bit0↔bit1).
            let mut snib = [0u8; 86];

            for k in 0..86 {
                // bit-reverse the 2-bit pair: swap bit0 and bit1
                let b0 = ((sector_data[k]      & 1) << 1) | ((sector_data[k]      >> 1) & 1);
                let b1 = ((sector_data[k + 86]  & 1) << 1) | ((sector_data[k + 86]  >> 1) & 1);
                let b2 = if k + 172 < 256 {
                    ((sector_data[k + 172] & 1) << 1) | ((sector_data[k + 172] >> 1) & 1)
                } else {
                    0
                };
                // pack into snib in reverse index order (snib[85], snib[84], ..., snib[0])
                snib[85 - k] = (b2 << 4) | (b1 << 2) | b0;
            }

            // Step 2: build the 256-byte primary nibble array (upper 6 bits)
            // Step 3: concatenate [snib(86 bytes), primary(256 bytes)] = nbuf(342 bytes)
            // Step 4: XOR-encode and table-lookup
            let mut last_val: u8 = 0;
            // encode snib first (indices 85 down to 0, already stored as snib[0..86])
            for i in 0..86 {
                let val6 = snib[i] & 0x3F;
                track_out.push(NIBBLE_WRITE_TABLE[(val6 ^ last_val) as usize]);
                last_val = val6;
            }
            // encode primary (upper 6 bits of each sector byte, in order)
            for i in 0..256 {
                let val6 = sector_data[i] >> 2;
                track_out.push(NIBBLE_WRITE_TABLE[(val6 ^ last_val) as usize]);
                last_val = val6;
            }
            // final checksum byte
            track_out.push(NIBBLE_WRITE_TABLE[last_val as usize]);

            track_out.push(0xDE); track_out.push(0xAA); track_out.push(0xEB);

            // Inter-sector gap: 27 self-sync bytes
            for _ in 0..27 { track_out.push(0xFF); }
        }
        tracks.push(track_out);
    }
    tracks
}
