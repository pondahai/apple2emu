# Gemini Context: Apple II Emulator (Rust)

This project is a cycle-accurate Apple II/II+ emulator written in Rust, focusing on low-level hardware fidelity and a clean separation between the emulation core and the platform-specific frontend.

## Project Architecture

The project is structured as a Rust workspace with two primary crates:

- **`apple2-core`**: The hardware emulation engine. Contains the MOS 6502 CPU, Memory Management Unit (MMU), Disk II Controller, and Video generation logic. It is designed for `no_std` compatibility to support embedded and WebAssembly targets.
- **`apple2-desktop`**: A Windows/Desktop frontend using `minifb` for windowing/graphics, `rodio` for audio, and `rfd` for native file dialogs.

## Key Technologies
- **Rust**: Primary language.
- **minifb**: Cross-platform windowing and framebuffer management.
- **rodio**: Real-time audio playback.
- **flate2**: Gzip support for compressed disk images (`.dsk.gz`).
- **arboard**: Clipboard integration for "pasting" text into the emulator.

## Building and Running

### Prerequisites
- **ROM Files**: Must be placed in the `roms/` directory.
    - `APPLE2PLUS.ROM` (12KB): Combined motherboard ROM.
    - `Apple II plus Video ROM - 341-0036 - Rev. 7.bin` (2KB): Character ROM.
    - `DISK2.ROM` (256 bytes): Disk II P5A Boot ROM.
- **Disk Images**: `MASTER.DSK` (DOS 3.3) is recommended for initial testing.

### Commands
- **Run Emulator**: `cargo run --bin apple2-desktop`
- **Run Tests**: `cargo test` (Primarily in `apple2-core`)
- **Utility Tools**:
    - `cargo run --bin verify_nibble`: Validates GCR/Nibble encoding/decoding.
    - `cargo run --bin save_smoke`: Verifies disk write-back stability.
- **ROM Helper**: `python roms/build_rom.py` (Merges 2KB ROM chunks into the 12KB main ROM).

## Development Conventions

### Hardware Implementation
- **CPU**: Implements the 6502 instruction set including common "illegal" opcodes (LAX, SAX, DCP, ISC, SLO, RLA, SRE, RRA). Illegal NOPs (SKB, SKW) are implemented to ensure correct memory access cycles.
- **Memory**: Accurate soft-switch mapping for I/O ($C000-$CFFF), including keyboard, speaker, and video mode switching.
- **Disk II**: Uses a state-machine sequencer. Timing is critical (32 CPU cycles per byte) for DOS 3.3 compatibility.
- **Audio**: Cycle-accurate speaker toggling with a high-pass DC filter and low-pass analog simulation.

### Frontend Features
- **Speed Control**: `F4` cycles through 1x, 1.2x, 1.5x, 2x, 5x, and "Unthrottled" speeds.
- **Auto-Turbo**: The emulator automatically enters "Unthrottled" mode when the Disk II motor is active to speed up loading.
- **Clipboard**: `Ctrl + V` (Windows) or `Right-Click` triggers a clipboard paste, injecting text into the keyboard latch.

## Current Project Status
- **Disk II**: Stable. Supports reading/writing and boots DOS 3.3.
- **CPU**: Mostly complete, but some bottlenecks remain in Stage 2 boot (specifically around `$0BB8`). Investigation into subtle flag differences or missing illegal opcode side effects is ongoing.
- **Graphics**: Text, Lo-Res, and Hi-Res (with NTSC artifact color) are fully implemented.

## Key Files
- `apple2-core/src/cpu.rs`: Main CPU fetch/execute loop and opcode dispatch.
- `apple2-core/src/instructions.rs`: Implementation of individual 6502 instructions.
- `apple2-core/src/disk2.rs`: Disk II controller logic and state machine.
- `apple2-desktop/src/main.rs`: Frontend loop, input handling, and audio mixing.
