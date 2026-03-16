fn format_operand(mode: &str, op1: u8, op2: u8, pc: usize) -> String {
    match mode {
        "impl" | "A" => String::new(),
        "#" => format!("#${:02X}", op1),
        "zpg" => format!("${:02X}", op1),
        "zpg,X" => format!("${:02X},X", op1),
        "zpg,Y" => format!("${:02X},Y", op1),
        "abs" => format!("${:04X}", op1 as u16 | (op2 as u16) << 8),
        "abs,X" => format!("${:04X},X", op1 as u16 | (op2 as u16) << 8),
        "abs,Y" => format!("${:04X},Y", op1 as u16 | (op2 as u16) << 8),
        "(ind,X)" => format!("(${:02X},X)", op1),
        "(ind),Y" => format!("(${:02X}),Y", op1),
        "rel" => format!("${:04X}", (pc as i32 + 2 + op1 as i8 as i32) as u16),
        "(ind)" => format!("(${:04X})", op1 as u16 | (op2 as u16) << 8),
        _ => "???".to_string(),
    }
}

fn decode_6502(opcode: u8) -> (&'static str, &'static str, usize) {
    match opcode {
        0x00 => ("BRK", "impl", 1),
        0x08 => ("PHP", "impl", 1),
        0x0A => ("ASL", "A", 1),
        0x18 => ("CLC", "impl", 1),
        0x20 => ("JSR", "abs", 3),
        0x24 => ("BIT", "zpg", 2),
        0x28 => ("PLP", "impl", 1),
        0x29 => ("AND", "#", 2),
        0x2A => ("ROL", "A", 1),
        0x38 => ("SEC", "impl", 1),
        0x48 => ("PHA", "impl", 1),
        0x4A => ("LSR", "A", 1),
        0x4C => ("JMP", "abs", 3),
        0x4E => ("LSR", "abs", 3),
        0x60 => ("RTS", "impl", 1),
        0x66 => ("ROR", "zpg", 2),
        0x68 => ("PLA", "impl", 1),
        0x6A => ("ROR", "A", 1),
        0x84 => ("STY", "zpg", 2),
        0x85 => ("STA", "zpg", 2),
        0x86 => ("STX", "zpg", 2),
        0x88 => ("DEY", "impl", 1),
        0x8A => ("TXA", "impl", 1),
        0x8D => ("STA", "abs", 3),
        0x90 => ("BCC", "rel", 2),
        0x98 => ("TYA", "impl", 1),
        0x99 => ("STA", "abs,Y", 3),
        0xA0 => ("LDY", "#", 2),
        0xA2 => ("LDX", "#", 2),
        0xA4 => ("LDY", "zpg", 2),
        0xA5 => ("LDA", "zpg", 2),
        0xA8 => ("TAY", "impl", 1),
        0xA9 => ("LDA", "#", 2),
        0xAA => ("TAX", "impl", 1),
        0xAC => ("LDY", "abs", 3),
        0xAD => ("LDA", "abs", 3),
        0xAE => ("LDX", "abs", 3),
        0xB0 => ("BCS", "rel", 2),
        0xB1 => ("LDA", "(ind),Y", 2),
        0xB9 => ("LDA", "abs,Y", 3),
        0xBD => ("LDA", "abs,X", 3),
        0xC5 => ("CMP", "zpg", 2),
        0xC6 => ("DEC", "zpg", 2),
        0xC8 => ("INY", "impl", 1),
        0xCA => ("DEX", "impl", 1),
        0xCC => ("CPY", "abs", 3),
        0xCE => ("DEC", "abs", 3),
        0xD0 => ("BNE", "rel", 2),
        0xE6 => ("INC", "zpg", 2),
        0xE8 => ("INX", "impl", 1),
        _ => ("???", "impl", 1),
    }
}

fn disasm(bytes: &[u8], base: usize) {
    let mut pc = 0usize;
    while pc < bytes.len() {
        let opcode = bytes[pc];
        let (mnemonic, mode, len) = decode_6502(opcode);
        let op1 = bytes.get(pc + 1).copied().unwrap_or(0);
        let op2 = bytes.get(pc + 2).copied().unwrap_or(0);
        println!(
            "${:04X}: {:02X} {:02X} {:02X}  {:<3} {}",
            base + pc,
            opcode,
            op1,
            op2,
            mnemonic,
            format_operand(mode, op1, op2, base + pc)
        );
        pc += len;
    }
}

fn main() {
    let block_0300: [u8; 0x18] = [
        0xA0, 0x20, 0x88, 0xF0, 0x61, 0xBD, 0x8C, 0xC0,
        0x10, 0xFB, 0x49, 0xD5, 0xD0, 0xF4, 0xEA, 0xBD,
        0x8C, 0xC0, 0x10, 0xFB, 0xC9, 0xAA, 0xD0, 0xF2,
    ];
    let block_05b2: [u8; 0x22] = [
        0x0A, 0x20, 0xBA, 0x05, 0x4E, 0x8E, 0x02, 0x60,
        0x85, 0xE4, 0x20, 0xCD, 0x05, 0xB9, 0x8E, 0x02,
        0x8D, 0x8E, 0x02, 0xA5, 0xE4, 0x99, 0x8E, 0x02,
        0x4C, 0x00, 0x04, 0x8A, 0x4A, 0x4A, 0x4A, 0x4A,
        0xA8, 0x60,
    ];

    println!("== $0300 ==");
    disasm(&block_0300, 0x0300);
    println!();
    println!("== $05B2 ==");
    disasm(&block_05b2, 0x05B2);
}
