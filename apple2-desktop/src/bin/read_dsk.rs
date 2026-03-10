use std::fs::File;
use std::io::Read;

fn main() {
    let mut f = File::open(r"C:\Users\Dell\Downloads\AppleWin1.30.18.0\MASTER.DSK").unwrap();
    let mut buf = [0u8; 16];
    f.read_exact(&mut buf).unwrap();
    
    print!("MASTER.DSK Sector 0 starts with: ");
    for b in buf.iter() {
        print!("{:02X} ", b);
    }
    println!("");
}
