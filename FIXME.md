# FIXME: Apple II Emulator Current Issues

## 1. DOS 3.3 Boot Loop / Monitor Drop Issue
- **Symptom**: The Apple II emulator successfully boots the Disk II Boot ROM (`$C600`) and the disk drive spins, but it eventually falls back into the system Monitor (displaying `*` and stopping at PC `FD1D` / `FD24`).
- **Context**: The `C600` ROM successfully pulls in Sector 0 of Track 0 into memory address `$0800`. However, the checksum validation or subsequent execution fails, causing the DOS 3.3 loader to retry reading. After too many retries (or an errant `BRK` jump), the system triggers an interrupt or halts.

## 2. Regression in `nibble.rs` (6-and-2 Data Encoding)
- **Symptom**: Memory at `$0800` is currently filled with corrupted data (`02 A5 24 CB 09 D2...` instead of the correct `01 A5 27 C9 09 D0...`).
- **Cause**: In the most recent code change to `apple2-core/src/nibble.rs`, the `(0..256).rev()` loop was removed under the assumption it was swapping bits incorrectly. **However, Apple DOS 3.3 hardware shifting (via ROL/LSR) actually requires the secondary bytes to be built in reverse order.** The original implementation using `.rev()` was correct in `nibble.rs`! Removing it broke the bit-packing, injecting illegal opcodes (`CB`, `A7`) into the bootloader.

## 3. CPU Flag edge-cases (ADC/SBC in Decimal Mode)
- **Symptom**: Before the `nibble.rs` regression, `$0800` data was correct, but the system still rejected the disk read. 
- **Cause**: Apple's DOS 3.3 RWTS heavily abuses the `ADC/SBC` operations and Carry/Zero/Negative Flags in Decimal Mode (BCD) to compute the sector checksums. The newly implemented BCD arithmetic in `cpu.rs` might still have an off-by-one or carry generation flaw under the very specific undocumented Edge Cases of the 6502 CPU.
