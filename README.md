# Apple II Emulator in Rust

A custom-built Apple II/II+ emulator written from scratch in Rust. It aims to accurately emulate the core components of the classic Apple II computer, focusing on low-level CPU operations, memory mapping, video generation, and Disk II controller logic.

## Project Structure

This emulator is split into a workspace with two main crates:

- **`apple2-core`**: The core library handling all hardware emulation. It is designed to be `no_std` compatible, allowing the core emulation logic to potentially run on embedded devices (like ESP32/RP2040) or WebAssembly without operating system dependencies.
- **`apple2-desktop`**: The Windows/Desktop GUI frontend. It uses the `minifb` crate for high-performance cross-platform windowing, frame buffering, and input handling. It also uses `arboard` for clipboard support.

## Features

### MOS 6502 CPU Emulation
- Full implementation of the 6502 instruction set.
- Accurate status flag logic (N, V, B, D, I, Z, C).
- Stack pushes/pops and interrupt handling (BRK).
- Trace logging for PC, Registers, and executing Code.

### Memory & Soft Switches (MMU)
- Accurate memory map mimicking the Apple II architecture:
  - `$0000` - `$BFFF`: 48KB Main RAM
  - `$C000` - `$CFFF`: Hardware I/O and Soft Switches
  - `$D000` - `$FFFF`: 12KB System ROM (Autostart & BASIC)
- Emulation of memory-mapped keyboard registers (`$C000` data, `$C010` clear strobe).
- Video soft switch intercepts (`$C050` - `$C057`) to toggle graphics modes and pages.

### Video & Graphics
- **Text Mode**: 40x24 monochrome text rendering using the original Apple II Character ROM. Supports Inverse and Flashing (blinking) text attributes.
- **Lo-Res Graphics (`GR`)**: 40x48 pixel rendering utilizing the authentic 15-color Apple II palette.
- **Hi-Res Graphics (`HGR`)**: 280x192 bitmapped rendering. Implements NTSC artifact color approximation, correctly decoding green, purple, blue, orange, black, and white based on the odd/even column placement and the bit 7 shift palette.

### Keyboard & Input
- Robust queue-based key delivery preventing dropped keystrokes.
- **Control Key Modifier**: Supports Apple II specific control sequences (like `Ctrl+B` to drop into BASIC from the Monitor).
- **Shift Key Modifier**: Translates symbols (e.g., `!`, `@`, `#`) correctly.
- **Clipboard Paste**: Press `Ctrl+V` to inject text directly from the host OS clipboard into the Apple II keyboard stream. Converts lowercase to uppercase automatically and maps newlines to Apple II Return (0x0D), allowing you to paste entire blocks of BASIC code instantly.

### Disk II Controller (Slot 6)
- Custom state machine emulating the Disk II sequencer.
- Cycle-accurate rotational delays (~32 CPU cycles per byte) satisfying the tight timing loops of the DOS 3.3 RWTS routines.
- Read sequencing and GCR (6-and-2 / 4-and-4) decoding capable of booting raw `.dsk` images.

## Requirements

To build and run this emulator, you need:

1. **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/).
2. **Apple II System ROMs**: 
   - `APPLE2PLUS.ROM` (12KB)
   - `341-0036.bin` (2KB Character ROM)
   - `DOS33_ROM.bin` (256 byte Disk Controller ROM)
3. **DOS 3.3 Disk Image**: e.g., `MASTER.DSK` (140KB).

*(Note: ROM and disk image paths are currently hardcoded in `apple2-desktop/src/main.rs`. Please update them to point to your local files).*

## Building and Running

Ensure your terminal is in the project's root workspace folder, then run:

```bash
cargo run --bin apple2-desktop
```

### Basic Usage

- **Booting**: The emulator resets to the normal boot vector. If a disk is loaded, it attempts to boot DOS 3.3.
- **Monitor**: To enter the Monitor manually from BASIC, type `CALL -151`.
- **BASIC**: To enter AppleSoft BASIC from the Monitor (`*`), type `Ctrl+B` and press `Enter`.
- **Pasting Code**: Copy any text from your computer, click the emulator window, and press `Ctrl+V`.

## License
Created as an experimental Rust emulation project.
