# TODO: Apple II Emulator Debugging Plan

## 1. Immediate Action: Revert `nibble.rs` 6-and-2 encoding
The previous loop `for i in (0..256).rev()` was correct per standard Apple II *Beneath Apple DOS* specifications.
- [ ] Change lines 118-126 in `apple2-core/src/nibble.rs` back to process `sector_data` in reverse (`.rev()`).
- [ ] Verify that `Memory at $0800` logs `01 A5 27 C9 09 D0...` again upon boot. This ensures the RWTS loader is correctly unpacking Sector 0.

## 2. Validate 6502 BCD Arithmetic (Decimal Mode)
The Apple DOS 3.3 RWTS uses a very strict checksum involving `SBC` and `ADC` under Decimal Mode (D=1).
- [ ] Review `apple2-core/src/instructions.rs` `adc` and `sbc` implementations against strict 6502 BCD hardware specifications. 
- [ ] Specifically verify the N (Negative), V (Overflow), and Z (Zero) flags are being updated properly in Decimal Mode, as the 6502 BCD implementation has known undocumented quirks (e.g., N, V, and Z are set based on binary additions *before* or *after* BCD adjustment depending on the exact chip mask).
- [ ] Consider adding a quick unit test to `apple2-core/src/instructions.rs` feeding known inputs and expected flag outputs from a 6502 test suite.

## 3. Verify Indirect Addressing Mode Quirks
If addressing modes are improperly emulated, the RWTS jumps into random memory space resulting in the `*` Monitor prompt.
- [ ] Verify the 6502 page-wrap bug is correctly applied to `JMP (Indirect)` where `0xXXFF` incorrectly wraps to `0xXX00`.
- [ ] Verify `IndirectY` (`AddressingMode::IndirectY`) correctly fetches the high byte and adds `Y`, ensuring no off-by-one errors happen if it crosses a page boundary. Apple II DOS uses this heavily for track/sector lookup indexing.

## 4. Ensure `BRK` and Interrupt Flags
The `FD1D / FD24` addresses in the Monitor usually indicate the Apple ID'd a break instruction (`0x00`) due to wild execution.
- [ ] Ensure `BRK` pushes the PC+1 or PC+2 correctly (currently PC+1). 6502 standard is PC+2 (skipping the signature byte).
- [ ] Make sure the `B` flag is correctly set on the Stack, but NOT in the CPU status register, as Apple II monitor directly reads `PHP` or `BRK` stack prints to show register dumps like `A=xx X=xx Y=xx P=xx S=xx`.
