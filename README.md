# Apple II Emulator in Rust

A custom-built Apple II/II+ emulator written from scratch in Rust. It aims to accurately emulate the core components of the classic Apple II computer, focusing on low-level CPU operations, memory mapping, video generation, and Disk II controller logic.

## Project Structure

This emulator is split into a workspace with two main crates:

- **`apple2-core`**: The core library handling all hardware emulation. It is designed to be `no_std` compatible, allowing the core emulation logic to potentially run on embedded devices (like ESP32/RP2040) or WebAssembly without operating system dependencies.
- **`apple2-desktop`**: The Windows/Desktop GUI frontend. It uses the `minifb` crate for high-performance cross-platform windowing, frame buffering, and input handling. It also uses `arboard` for clipboard support, and `rodio` for real-time audio playback.

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

### Audio Emulation
- Toggles the speaker state precisely via the `$C030` memory-mapped I/O port.
- Tracks exact cycle counts between toggles to generate cycle-accurate audio in real-time.
- Employs a custom High-pass filter (DC Blocker) to quickly decay continuous voltage when the speaker is idle, eliminating annoying continuous pop/crack sounds.
- Runs at a robust 22,050 Hz sample rate with fractional CPU cycle phase-tracking across audio frames for smoother beep generation. 
- Implements automated audio buffer padding to prevent chopped audio playback during heavy CPU or rendering loads.

### Disk II Controller (Slot 6)
- Custom state machine emulating the Disk II sequencer.
- Accurate quarter-track magnetic phase stepper motor emulation.
- Cycle-accurate rotational delays (~32 CPU cycles per byte) satisfying the tight timing loops of the DOS 3.3 RWTS routines.
- Read sequencing and GCR (6-and-2 / 4-and-4) decoding capable of fully booting DOS 3.3 raw `.dsk` images into Applesoft BASIC.
- Write sequencing supports DOS `SAVE` flow and any write operations to the disk.
- **Disk Write-Back (Save)**: Any changes written to the disk are automatically saved back to the `.dsk` or `.gz` file when the disk is ejected (`F3`), upon emulator reboot (`F2`), or when closing the application.
- **Automatic Disk Turbo**: whenever the Disk II motor is spinning, the desktop frontend temporarily removes frame throttling to speed up disk reads and seeks; when the motor stops, speed returns to the current manual `F4` setting.
- **Dynamic Disk Loading**: Press `F3` to open a file selection dialog. Supports standard `.dsk`, `.do`, `.po` images and Gzip compressed `.gz` images.

## Requirements

To build and run this emulator, you need:

1. **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/).
2. **Apple II ROM Files** — place all in the `roms/` folder (see `SETUP.md` for sources):
   - `APPLE2PLUS.ROM` (12KB — Apple II+ Motherboard ROM)
   - `Apple II plus Video ROM - 341-0036 - Rev. 7.bin` (2KB — Character ROM)
   - `DISK2.ROM` (256 bytes — Official Disk II Controller ROM, P5A / 341-0027)
   - `DISK2_P6.ROM` (256 bytes — Official State Machine ROM, 341-0028 - reserved for future use)
3. **DOS 3.3 Disk Image**: `MASTER.DSK` (140KB) — place in `roms/MASTER.DSK`

All paths are **relative** (`../roms/...`) — no hardcoded absolute paths needed.
See `SETUP.md` for full instructions on where to get each file.

## Building and Running

Ensure your terminal is in the project's root workspace folder, then run:

```bash
cargo run --bin apple2-desktop
```

### Hotkeys & Basic Usage

- **`F2`**: **Reboot**. Restarts the emulator and performs a clean boot from disk (simulates a power-on).
- **`Ctrl + Delete`**: **System Reset**. Simulates the physical `Reset` key on an Apple II (warm reset).
- **`F3`**: **Load Disk Image**. Opens a system file dialog to choose a `.dsk` or `.gz` disk image.
- **`F4`**: **Speed Cycle**. Cycles CPU speed through **1x → 2x → 3x → 4x → 5x → 1x** and relaxes frame/audio/debug overhead when above 1x.
- **Disk Motor Auto Turbo**: While the Disk II motor is on, the window title shows **`AUTO TURBO UNTHROTTLED`** and the frontend runs unthrottled. When the motor stops, it falls back to the current `F4` speed mode.
- **Joystick**: Arrow keys drive Paddle 0/1 (X/Y). `Left Alt` maps to Pushbutton 0, `Right Alt` maps to Pushbutton 1.
- **`Ctrl + V`**: **Paste Text**. Inject text from your host clipboard directly into the Apple II keyboard stream.
- **Monitor**: To enter the Monitor manually from BASIC, type `CALL -151`.
- **BASIC**: To enter AppleSoft BASIC from the Monitor (`*`), type `Ctrl+B` and press `Enter`.

## License
Created as an experimental Rust emulation project.
