# Apple II Emulator in Rust

An Apple II+ emulator written from scratch in Rust: 6502 CPU, 48K RAM plus a 16K Language Card, text / lo-res / hi-res video, speaker audio, joystick, and a Disk II controller that boots and writes DOS 3.3 disks.

## Project Structure

The workspace has two crates:

- **`apple2-core`**: all hardware emulation (CPU, memory map and soft switches, video rendering, Disk II, nibble encoding). It has no windowing or audio dependencies, but it is **not** `no_std` today: it still uses `std` for debug `println!` output and for the text-blink timer.
- **`apple2-desktop`**: the Windows desktop frontend. It uses `minifb` for the window, framebuffer and keyboard, `rodio` for audio, `arboard` for the clipboard, `rfd` for file and message dialogs, and `directories` + `serde_json` for the settings file. It also contains a few diagnostic tools under `src/bin/`.

## Features

### 6502 CPU
- All documented NMOS 6502 instructions with cycle counts, including decimal (BCD) mode.
- Undocumented opcodes as well (LAX, SAX, DCP, ISC, SLO, RLA, SRE, RRA, ANC, ALR, ARR, SBX, the unstable SHA/SHX/SHY/TAS/XAA group, and KIL/JAM, which halts the CPU until reset).
- `BRK` vectors through `$FFFE`. There are no IRQ or NMI sources, because the emulated machine has no cards that would raise them.

### Memory Map & Soft Switches
- `$0000`-`$BFFF`: 48K main RAM.
- `$C000`-`$CFFF`: I/O space:
  - keyboard: `$C000` data, `$C010` strobe clear
  - speaker: `$C030`
  - video switches: `$C050`-`$C057`
  - pushbuttons: `$C061`/`$C062`
  - paddles: `$C064`-`$C067`, with the `$C070` timer reset
  - Language Card switches: `$C080`-`$C08F`
  - Disk II: `$C0E0`-`$C0EF`, with the slot 6 boot ROM at `$C600`
- `$D000`-`$FFFF`: 12K system ROM, or the **16K Language Card** RAM (two 4K banks at `$D000`, following the standard double-read write-enable protocol).
- **Floating bus**: reads of undriven I/O locations return the byte the video scanner is currently fetching, as on real hardware.

### Video
- **Text**: 40x24 characters drawn with the original character ROM, including inverse and flashing characters.
- **Lo-res** (`GR`): 40x48 blocks in the 16-color palette.
- **Hi-res** (`HGR`): 280x192 with NTSC artifact colors (black, white, green, purple, orange, blue), chosen from each pixel's column parity and bit 7 of its byte.
- **Mixed mode** (4 lines of text below the graphics) and **page 1/2** switching.
- **`F7`** toggles a green monochrome-monitor look.

### Keyboard, Paste & Joystick
- Key presses are captured from the OS event stream, so a quick tap between two frames is never lost. A held key auto-repeats at about 15 characters per second.
- Letters are always sent as uppercase. `Ctrl`+letter sends control codes (for example `Ctrl+B` for BASIC from the Monitor). `Shift`+digit and `Shift`+punctuation send the shifted symbols. `Enter` sends Return, `Backspace` sends ← (`Ctrl+H`) and `Esc` sends Escape.
- **Paste**: **click the right mouse button** to type the host clipboard into the Apple II. Lowercase is converted to uppercase and newlines to Return, and characters are fed only as fast as the program reads the keyboard, so long BASIC listings paste reliably.
- **Joystick**: the arrow keys drive paddles 0/1 (X/Y). They act only as the joystick, not as keyboard keys. **`Right Alt` is pushbutton 0** and **`Left Alt` is pushbutton 1**.

### Audio
- Speaker clicks (`$C030`) are timestamped to the exact CPU cycle and mixed into 44.1 kHz audio, followed by a DC-blocking high-pass filter so an idle speaker makes no pop or hum.
- Audio playback is paced to real time. If playback falls behind (a stall, or disk auto turbo), the stale backlog is dropped instead of playing late. At a fixed turbo speed, audio is pitch-shifted to match the emulation speed.
- **`F8` / `F9`**: volume down / up in 10% steps. The window title shows `[VOL n%]` or `[MUTED]`.

### Disk II Controller (Slot 6)
- Bit-level read/write shift register timed at 4 CPU cycles per bit (32 per byte).
- Four-phase stepper motor with quarter-track head positioning, a 1-second motor-off delay and spin-up time.
- Disk images are converted to a nibble stream (DOS 3.3 sector interleave, 4-and-4 address fields, 6-and-2 data fields) and converted back when saving, so DOS 3.3 `SAVE`, `INIT` and other writes work.
- **Supported images**: 143,360-byte **DOS-order** images (`.dsk` / `.do`), optionally gzip-compressed (`.gz`, detected by content). ProDOS-order (`.po`) and `.nib` / `.woz` images are **not** supported.
- **Automatic disk turbo**: while the drive motor is on, emulation runs unthrottled to speed up loading, then returns to the selected `F5` speed.
- **Write-back**: a modified disk is saved back to its file (re-compressed if it is `.gz`) when you cold reboot with `F2`, when you quit, and when you swap disks with `F3`. The `F3` save applies only if the current disk was itself opened with `F3`.

### Memory Monitor (`F6`)
- Pauses emulation and opens an **on-screen 80-column hex viewer** over the whole 64K address space.
- It opens **read-only**:
  - Move with the arrow keys, `PgUp` / `PgDn` and `Home` / `End`.
  - Press `G` to go to an address.
  - Press **`Enter`** to switch to edit mode, where typing hex digits overwrites bytes. Press `Enter` or `Esc` to return to viewing.
  - Press `Esc` or `F6` to resume emulation.
- While paused, the **console window** also accepts Apple II Monitor-style commands: `300` (examine), `300.3FF` (dump), `300:A9 00 60` (store), `:EA` (continue storing), `?` (help) and `Q` (resume).
- Reading memory here never triggers soft switches. I/O space (`$C000`-`$CFFF`) cannot be edited, and writes to `$D000`-`$FFFF` go into the currently selected Language Card bank.

### Settings
The last disk path, volume, speed step and color/green mode are saved to `config.json` in the per-user config folder (on Windows, `%APPDATA%\Apple2Emu\Apple2Emu\config\`) and restored at the next launch.

## Requirements

1. **Rust toolchain**: install it with [rustup.rs](https://rustup.rs/).
2. **ROM files** in the `roms/` folder. See `SETUP.md` for where to get them:
   - `APPLE2PLUS.ROM`: 12K Apple II+ system ROM (`$D000`-`$FFFF`). **Required**: the emulator exits with an error dialog if it is missing.
   - `DISK2.ROM`: 256-byte Disk II boot ROM (P5, 341-0027). Without it, floppies cannot boot.
   - `Apple II plus Video ROM - 341-0036 - Rev. 7.bin`: 2K character ROM. Without it, text is drawn as a checkerboard.
3. **Boot disk**: `roms/MASTER.DSK` (a DOS 3.3 image). It is used only when no previously loaded disk is remembered in the settings.

Missing optional files are listed in a single warning dialog at startup. The `roms/` folder is looked up next to the build output (`target/<profile>/../../roms`), then in `./roms`, then in `../roms`.

## Building and Running

From the workspace root:

```bash
cargo run --bin apple2-desktop
```

### Hotkeys

| Key | Function |
|---|---|
| `F1` | Warm reset (like Ctrl-Reset: resets only the CPU, and RAM and the disk stay as they are) |
| `F2` | Cold reboot: saves a modified disk, clears the machine and boots again. `Ctrl+F2` does a warm reset instead |
| `F3` | Load a disk image (file dialog filters `.dsk` / `.gz`) |
| `F5` | Cycle speed: **1x → 1.2x → 1.5x → 2x → 5x → unthrottled → 1x** (shown in the window title) |
| `F6` | Memory monitor (see above) |
| `F7` | Toggle color / green monochrome |
| `F8` / `F9` | Volume down / up |
| `F10` | Quit (closing the window also works). A modified disk is saved first |
| Right mouse button | Paste the clipboard as keystrokes |
| Arrow keys, `Right Alt` / `Left Alt` | Joystick, pushbutton 0 / pushbutton 1 |

`F4` is currently unused.

From BASIC, `CALL -151` enters the built-in Apple II Monitor. From the Monitor (`*`), `Ctrl+B` then `Enter` returns to Applesoft BASIC.

## License
Created as an experimental Rust emulation project.
