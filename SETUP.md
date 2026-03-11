# Apple II Emulator Setup Notes

This file documents which ROM files are needed and where to get them.
These ROM files are NOT committed to git (see .gitignore).

## Required ROM Files (place all in `roms/` folder)

| File | Size | Source | Description |
|------|------|--------|-------------|
| `APPLE2PLUS.ROM` | 12288 bytes | Built from 6x 2KB chunks below | Apple II+ main ROM (D000-FFFF) |
| `Apple II plus Video ROM - 341-0036 - Rev. 7.bin` | 2048 bytes | See below | Character generator ROM |
| `MASTER.DSK` | 143360 bytes | AppleWin 1.26 folder | DOS 3.3 master disk image |
| `extracted_256_150.bin` | 256 bytes | See below | Disk II controller card ROM (P5A) |

## Where to Get These Files

### APPLE2PLUS.ROM (12KB main ROM)
Built by merging 6 individual 2KB pages from:
- `Apple II plus ROM Pages D0-D7 - 341-0011 - Applesoft BASIC.bin`
- `Apple II plus ROM Pages D8-DF - 341-0012 - Applesoft BASIC.bin`
- `Apple II plus ROM Pages E0-E7 - 341-0013 - Applesoft BASIC.bin`
- `Apple II plus ROM Pages E8-EF - 341-0014 - Applesoft BASIC.bin`
- `Apple II plus ROM Pages F0-F7 - 341-0015 - Applesoft BASIC.bin`
- `Apple II plus ROM Pages F8-FF - 341-0020 - Autostart Monitor.bin`

Run `roms/build_rom.py` to merge them: `python roms/build_rom.py`

These individual ROM pages can be downloaded from:
https://mirrors.apple2.org.za/ftp.apple.asimov.net/emulator/rom_images/

### Apple II+ Video ROM (341-0036)
Same source as above - look for the Character ROM file.

### MASTER.DSK (DOS 3.3 disk image)
Available from the AppleWin installation folder:
- `C:\Users\pondahai\Downloads\AppleWin1.26.1.1\MASTER.DSK`

### Disk II Controller ROM (extracted_256_150.bin, 256 bytes)
This is the **P5A** Disk II Interface card boot ROM (341-0027-A).
Standard well-known bytes starting with: `A2 20 A0 00 A2 03 86 3C ...`

To extract from AppleWin.exe (AppleWin 1.26):
```
python roms/extract_disk_rom.py
```
(See roms/extract_disk_rom.py for the extraction logic)

**Note**: This ROM content is historically documented and the byte sequence is
consistent across all legitimate Apple II emulators. First byte = 0xA2 (LDX #$20).

## Quick Setup Steps

1. Copy `MASTER.DSK` from AppleWin folder to `roms/MASTER.DSK`
2. Download the individual Apple II+ ROM pages and run `python roms/build_rom.py`
3. Download or extract `extracted_256_150.bin` (256 bytes, Disk II P5A ROM)
4. Run: `cargo run --bin apple2-desktop` from the `apple2-desktop/` folder

## Key File Paths in main.rs

All paths in `apple2-desktop/src/main.rs` use paths **relative to the apple2-desktop/ folder**:
- Main ROM: `../roms/APPLE2PLUS.ROM`
- Character ROM: `../roms/Apple II plus Video ROM - 341-0036 - Rev. 7.bin`
- Disk II ROM: `../roms/extracted_256_150.bin`
- Startup disk: `../roms/MASTER.DSK`
