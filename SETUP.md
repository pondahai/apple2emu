# Apple II Emulator Setup Notes

This file documents which ROM files are needed and where to get them.
These ROM files are NOT committed to git (see .gitignore).

Verified working end-to-end on 2026-09-25 - DOS 3.3 boots all the way to the
`]` prompt with `roms/MASTER.DSK`. See "Setup Log" below for the full story,
including a ROM-content bug that took the longest to track down.

## Required Files (place all in `roms/` folder)

| File | Size | Description |
|------|------|-------------|
| `APPLE2PLUS.ROM` | 12288 bytes | Apple II+ main ROM (D000-FFFF) |
| `DISK2.ROM` | 256 bytes | Disk II controller card (Slot 6) boot ROM, P5A / 341-0027-A |
| `MASTER.DSK` | 143360 bytes | DOS 3.3 boot disk image |
| `Apple II plus Video ROM - 341-0036 - Rev. 7.bin` | 2048 bytes | Character generator ROM (optional - without it, text renders as a checkerboard but the machine still boots) |

**Note:** The exact filenames above are what `apple2-desktop/src/main.rs` looks
for under the `roms/` folder next to the exe (see the `roms_dir.join(...)`
calls in `main.rs`). Older notes in this repo referred to
`extracted_256_150.bin` for the disk ROM - that name is stale; the code
actually loads `DISK2.ROM`.

Only `APPLE2PLUS.ROM` is fatal if missing (shows an error dialog and exits).
The other three just print a warning and the emulator still starts - but
without a good `DISK2.ROM` and `APPLE2PLUS.ROM` the machine will boot to a
banner/prompt and look fine while being silently broken (see below).

## Best source: PicoApple2's verified ROM set

This repo is a sibling project to `PicoApple2` (same author, same core
architecture), and PicoApple2 ships a **pre-verified** ROM set as a
downloadable zip for its own build (see PicoApple2's own setup docs/releases
for `pico_apple2_roms.zip`). If you have access to it, it's the fastest and
most reliable path - just copy across:

| PicoApple2 file | apple2emu file |
|---|---|
| `apple2_sys.rom` (12288 bytes) | `roms/APPLE2PLUS.ROM` |
| `apple2_char.rom` (2048 bytes) | `roms/Apple II plus Video ROM - 341-0036 - Rev. 7.bin` |
| `disk2_p5.rom` (256 bytes) | `roms/DISK2.ROM` |

(`disk2_p6.rom` from that set is not used by apple2emu - it's a disk
controller state-machine ROM the PicoApple2 firmware doesn't currently
reference either.)

`MASTER.DSK` isn't part of that set (copyright reasons); get it separately,
see below.

## Where to Get These Files From a Mirror (if you don't have the PicoApple2 set)

All of these are copyrighted Apple ROM/DOS images. Download them yourself
from a mirror such as:

https://mirrors.apple2.org.za/ftp.apple.asimov.net/emulators/rom_images/

(Note: it's `emulators/rom_images/`, not `emulator/rom_images/` as an
earlier draft of this file said - the singular path 404s and bounces you
back to the site root.)

### APPLE2PLUS.ROM - **watch out for a corrupted IRQ vector**
`apple_ii+_rom.zip` from the folder above unzips to a single `APPLE2.ROM`
(20480 bytes). The loader takes the *last* 12288 bytes of whatever you give
it, so this works size-wise. **However**, the copy pulled from that mirror on
2026-09-25 had a corrupted last byte: reset vector ($FFFC/$FFFD) read
correctly as `$FA62`, but the IRQ/BRK vector ($FFFE/$FFFF) was `$FF59`
instead of the correct `$FA40`. The machine still "boots" - ROM loads, beep
plays, banner prints - but the instant anything triggers a BRK/IRQ during
boot, the CPU jumps into garbage and the emulator ends up spinning at PC
`$0000` forever. From the desktop app this looked like the screen freezing
on the "APPLE ][" banner with no further progress, even after 20+ seconds.

**Verify before trusting any APPLE2PLUS.ROM**: the last two bytes of the
12KB image (offset `0x2FFE`-`0x2FFF`) should be `40 FA` (IRQ vector
`$FA40`), and the two bytes before that (`0x2FFC`-`0x2FFD`) should be
`62 FA` (reset vector `$FA62`). If your file ends any other way, get a
different copy - the PicoApple2 `apple2_sys.rom` is confirmed correct.

`roms/build_rom.py` still exists for the alternate path of merging 6
individual 2KB page dumps (`341-0011` through `341-0020`) if that's what you
have instead.

### DISK2.ROM (Disk II boot ROM, 256 bytes) - **don't trust a naive bit-fix**
In the same `rom_images/` folder, the file
`Apple Disk II 16 Sector Interface Card ROM P5 - 341-0027.bin` (256 bytes)
looks right by name and size, but its address/data lines are scrambled
compared to what a straight linear ROM dump should read - a common artifact
of some EPROM reader/dumper tools. Loading it as-is loads fine and even
*looks* like it works (correct size, "loads" without error) but the
Autostart ROM's slot scan never recognizes it as a valid boot device, so the
machine falls through to a `]`/BASIC-ish prompt instead of booting DOS 3.3.

The real boot code starts `A2 20 A0 00 A2 03 86 3C ...` (`LDX #$20 / LDY
#$00 / LDX #$03 / STX $3C ...`). The scrambled dump from this mirror starts
`52 40 50 00 52 03 16 CC ...`.

**Do not assume this is a simple bit-swap you can fix with a formula.** An
earlier pass through this file found that reversing the bit order of each
byte's upper nibble (D4<->D7, D5<->D6) happens to reconstruct the *first 32
bytes* correctly - which looked like a confirmed fix at the time - but the
remaining 224 bytes diverge from the real ROM under that same transform
(the scrambling also permutes address lines, not just data bits). Don't
trust a partial byte-match as proof the whole file is fixed; diff the full
256 bytes against a known-good copy (e.g. PicoApple2's `disk2_p5.rom`) if
you're not 100% sure.

The reliable fixes, in order of preference:
1. Use PicoApple2's `disk2_p5.rom` directly (see above) - already verified
   byte-for-byte.
2. `roms/extract_disk_rom.py` pulls this ROM out of an installed
   `AppleWin.exe` by byte-signature search, which sidesteps the scrambling
   entirely since AppleWin stores it already de-scrambled.
3. The mirror also lists a second file, `...-with-D4-D7 data bits
   swapped.bin` - untested here, but worth trying and diffing against a
   known-good copy before trusting it.

### MASTER.DSK (DOS 3.3 disk image)
Any legitimate 140KB (143360 byte) DOS 3.3 master disk image works, e.g.
`MASTER.DSK` from an AppleWin install, or a dump such as
"Apple DOS 3.3 January 1983.dsk". Copy it to `roms/MASTER.DSK`. This file
was fine from the mirror source in this session - no corruption found here.

### Character ROM (341-0036, 2048 bytes)
Not in the `rom_images/` folder as a standalone file. It's bundled inside
`apple2_roms.zip` (also in the same `rom_images/` folder) → `ROMS.ZIP` →
`3410036.BIN`. Rename that to
`Apple II plus Video ROM - 341-0036 - Rev. 7.bin` and copy to `roms/`. This
one matched PicoApple2's `apple2_char.rom` byte-for-byte, so it's a fine
source.

## Quick Setup Steps

1. If you have PicoApple2's `pico_apple2_roms.zip`, just copy `apple2_sys.rom` -> `roms/APPLE2PLUS.ROM`, `apple2_char.rom` -> `roms/Apple II plus Video ROM - 341-0036 - Rev. 7.bin`, `disk2_p5.rom` -> `roms/DISK2.ROM`, and skip to step 3.
2. Otherwise: download `apple_ii+_rom.zip` -> unzip -> copy `APPLE2.ROM` to `roms/APPLE2PLUS.ROM` (verify the last 2 bytes are `40 FA`, see above); download `Apple Disk II 16 Sector Interface Card ROM P5 - 341-0027.bin` to `roms/DISK2.ROM` (verify against a known-good copy, don't trust a partial byte match); download `apple2_roms.zip` -> unzip -> unzip `ROMS.ZIP` -> copy `3410036.BIN` to `roms/Apple II plus Video ROM - 341-0036 - Rev. 7.bin`
3. Get a DOS 3.3 disk image -> copy to `roms/MASTER.DSK`
4. Run `run.bat`, or `cargo run --release --bin apple2-desktop`
5. Sanity check before trusting the boot: `cargo run --release --bin boot_smoke` runs a headless 15-second boot and prints the seek sequence and final screen text. A healthy boot shows dozens of track seeks (0 through 20+) and ends with the DOS 3.3 banner text on screen. A broken ROM shows exactly one seek (`T0@0.0s`) and `final pc: 0000` with a huge single-PC hot-spot count - that's the CPU jammed, not a slow boot, no amount of waiting fixes it.

## Setup Log (2026-09-25)

Kept for context on what actually went wrong, in case it happens again:

1. **Missing build environment.** This machine had no Rust toolchain at
   all. Installed via `winget install --id Rustlang.Rustup`, which put
   `cargo`/`rustc` in `~/.cargo/bin`.
2. **No MSVC linker.** `cargo build --release` failed with a `link.exe`
   error because Visual Studio Build Tools weren't installed. Rather than
   pull down the multi-GB VS Build Tools, installed the GNU toolchain
   instead: `winget install --id BrechtSanders.WinLibs.POSIX.UCRT` (MinGW
   gcc/ld) + `rustup toolchain install stable-x86_64-pc-windows-gnu` +
   `rustup default stable-x86_64-pc-windows-gnu`. Build succeeded after
   that.
3. **Smart App Control blocked the freshly-built exe.** Running the
   self-compiled `apple2-desktop.exe` failed with "An Application Control
   policy has blocked this file" (Code Integrity event ID 3077/3118 in the
   `Microsoft-Windows-CodeIntegrity/Operational` log). This machine has
   Smart App Control turned on (`HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy!VerifiedAndReputablePolicyState
   = 1`), which blocks any unsigned/unrecognized binary with no allowlist
   option. Fixed by turning it off: Settings -> Privacy & security ->
   Windows security -> App & browser control -> Smart App Control -> Off.
   **This is a one-way door** - Microsoft's documented way back on is a
   clean Windows reinstall, so don't flip this without accepting that
   tradeoff.
4. **Disk ROM looked fixed but wasn't.** A bit-reversal transform applied to
   the mirror's scrambled `DISK2.ROM` reconstructed the first 32 bytes
   correctly, which was mistaken for a full fix at the time (booted, beeped,
   showed a cursor). It wasn't - the Autostart ROM never actually recognized
   the disk as bootable with that file, so it silently skipped straight to
   BASIC/monitor instead of loading DOS. This wasn't caught until later
   testing showed the boot behavior was inconsistent.
5. **Main ROM had a corrupted IRQ vector**, unrelated to the disk ROM issue.
   Even after swapping in a correct `DISK2.ROM`, the emulator would
   intermittently freeze on the "APPLE ][" banner with the CPU spinning at
   PC `$0000` (confirmed headless with `boot_smoke`, and confirmed
   reproducible even on an unmodified `git stash`ed checkout - i.e. not
   caused by any in-session code changes). Root cause: the mirror's
   `apple_ii+_rom.zom` → `APPLE2.ROM` had `$FFFE/$FFFF` (IRQ/BRK vector) set
   to `$FF59` instead of the correct `$FA40`; any interrupt during boot sent
   the CPU into garbage. Fixed by replacing both `APPLE2PLUS.ROM` and
   `DISK2.ROM` with PicoApple2's pre-verified `apple2_sys.rom` /
   `disk2_p5.rom` (byte-identical to the mirror copies for everything except
   the corrupted bytes). After the swap, `boot_smoke` shows a full multi-track
   boot sequence and lands on the real DOS 3.3 banner + `]` prompt.

**Takeaway:** a ROM file that "loads" (right size, no read error) and even
produces a plausible-looking boot screen is not proof it's byte-correct.
When something is inconsistent or the machine won't get past a splash
screen, diff the suspect ROM against a known-good source rather than
guessing at a bit-transform - and use `boot_smoke` (fast, headless, prints
the seek sequence) to check before spending time staring at the GUI window.

## Key File Paths in main.rs

Paths are resolved relative to a `roms/` folder that `main.rs` locates next
to the running executable (see `roms_dir` construction near the top of
`apple2-desktop/src/main.rs`):
- Main ROM: `roms/APPLE2PLUS.ROM`
- Disk II ROM: `roms/DISK2.ROM`
- Startup disk: `roms/MASTER.DSK` (or whatever `config.json`'s
  `last_disk_path` points at, once you've loaded a disk via F3)
- Character ROM: `roms/Apple II plus Video ROM - 341-0036 - Rev. 7.bin`

## In-App Keys (apple2-desktop)

Matches PicoApple2's F-key layout except volume, which apple2emu moved to
free up F6/F7 for other things:

| Key | Function |
|---|---|
| F1 | Warm reset (Ctrl-Reset: resets the CPU only, no RAM/disk reload) |
| F2 | Cold reboot (reloads ROM/disk state); Ctrl+F2 does a warm reset too |
| F3 | Load another disk image |
| F5 | Cycle emulation speed (1x/1.2x/1.5x/2x/5x/unthrottled) |
| F7 | Toggle color / green-screen monochrome display |
| F8 / F9 | Volume down / up (PicoApple2 uses F6/F7 for this instead) |
| F10 | Quit |

F4 and F6 are currently unbound.
