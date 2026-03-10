use std::fs::File;
use std::io::Read;

fn main() {
    let mut f = File::open("../roms/APPLE2PLUS.ROM").unwrap();
    let mut rom = [0u8; 12288];
    f.read_exact(&mut rom).unwrap();
    
    // Reset vector
    let reset = (rom[0x2FFD] as u16) << 8 | rom[0x2FFC] as u16;
    println!("Reset vector: ${:04X}", reset);
    
    // $FF59
    let ff59 = 0xFF59 - 0xD000;
    print!("$FF59 (Reset handler): ");
    for i in 0..20 { print!("{:02X} ", rom[ff59 + i]); }
    println!();
    
    // Check for LDA $03F4 (power-up byte check)
    let mut found = false;
    for i in 0..rom.len()-3 {
        if rom[i] == 0xAD && rom[i+1] == 0xF4 && rom[i+2] == 0x03 {
            let addr = 0xD000 + i;
            println!("✓ Found LDA $03F4 at ${:04X}", addr);
            found = true;
        }
    }
    if !found { println!("✗ No LDA $03F4"); }
    
    // Check for EOR #$A5
    for i in 0..rom.len()-2 {
        if rom[i] == 0x49 && rom[i+1] == 0xA5 {
            let addr = 0xD000 + i;
            println!("✓ Found EOR #$A5 at ${:04X}", addr);
        }
    }
    
    // Check for JMP ($03F2) 
    for i in 0..rom.len()-3 {
        if rom[i] == 0x6C && rom[i+1] == 0xF2 && rom[i+2] == 0x03 {
            let addr = 0xD000 + i;
            println!("✓ Found JMP ($03F2) at ${:04X}", addr);
        }
    }
    
    // $E000
    let e000 = 0xE000 - 0xD000;
    print!("$E000: ");
    for i in 0..8 { print!("{:02X} ", rom[e000 + i]); }
    println!();
    
    // $D000  
    print!("$D000: ");
    for i in 0..8 { print!("{:02X} ", rom[i]); }
    println!();
    
    // All regions
    for region in 0..6 {
        let start = region * 2048;
        let end = start + 2048;
        let non_zero = rom[start..end].iter().filter(|&&b| b != 0).count();
        println!("Region ${:04X}: {} non-zero bytes", 0xD000 + start, non_zero);
    }
}
