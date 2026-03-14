use apple2_core::nibble::NIBBLE_WRITE_TABLE;

fn main() {
    let dsk_path = "roms/MASTER.DSK";
    let disk_data = std::fs::read(dsk_path).expect("Cannot read roms/MASTER.DSK");
    let mut gcr_decode = [0xFFu8; 256];
    for (i, &gcr) in NIBBLE_WRITE_TABLE.iter().enumerate() {
        gcr_decode[gcr as usize] = i as u8;
    }

    use apple2_core::nibble::nibblize_dsk;
    let tracks = nibblize_dsk(&disk_data);
    let raw = &tracks[0].raw_bytes[..tracks[0].length];

    let mut data_start = 0;
    for i in 0..raw.len() - 3 {
        if raw[i] == 0xD5 && raw[i + 1] == 0xAA && raw[i + 2] == 0xAD {
            data_start = i + 3;
            break;
        }
    }

    for xor_mode in 0..2 {
        for seed in [0x00, 0xAD] {
            let mut raw_nibbles = [0u8; 342];
            let mut last = seed;
            for k in 0..342 {
                let val6 = gcr_decode[raw[data_start + k] as usize];
                if xor_mode == 0 {
                    let d = val6 ^ last;
                    raw_nibbles[k] = d;
                    last = d;
                } else {
                    raw_nibbles[k] = val6 ^ last;
                    last = val6;
                }
            }

            for sec_rev in [false, true] {
                for pri_rev in [false, true] {
                    for swap in [false, true] {
                        for snib_off in 0..86 {
                            for pnib_off in 0..256 {
                                let mut sector = [0u8; 256];
                                let mut snib = [0u8; 86];
                                for k in 0..86 {
                                    snib[k] = if sec_rev {
                                        raw_nibbles[85 - k]
                                    } else {
                                        raw_nibbles[k]
                                    };
                                }
                                let mut pnib = [0u8; 256];
                                for k in 0..256 {
                                    pnib[k] = if pri_rev {
                                        raw_nibbles[341 - k]
                                    } else {
                                        raw_nibbles[86 + k]
                                    };
                                }

                                for k in 0..256 {
                                    let s = snib[(k + snib_off) % 86];
                                    let p = pnib[(k + pnib_off) % 256];
                                    let mut b = if k < 86 {
                                        (s >> 4) & 3
                                    } else if k < 172 {
                                        (s >> 2) & 3
                                    } else {
                                        s & 3
                                    };
                                    if swap {
                                        b = ((b & 1) << 1) | ((b >> 1) & 1);
                                    }
                                    sector[k] = (p << 2) | b;
                                }
                                if sector[0] == 0x01 && sector[1] == 0xA5 && sector[2] == 0x27 {
                                    println!(
                                        "FOUND! XOR:{}, Seed:{:02X}, SecRev:{}, PriRev:{}, SOff:{}, POff:{}, Swap:{}",
                                        xor_mode, seed, sec_rev, pri_rev, snib_off, pnib_off, swap
                                    );
                                    print!("Data: ");
                                    for b in &sector[..8] {
                                        print!("{:02X} ", b);
                                    }
                                    println!();
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
