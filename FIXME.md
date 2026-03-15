# FIXME: Apple II Emulator — Known Issues

## 1. Bit-Level Disk Synchronization

**Context**: Current Disk II emulation works at the byte level (~32 cycles per byte). 
However, some RWTS routines and copy protections rely on the **P6 Sequencer state machine** 
to handle sub-byte bit slips and 10-bit sync patterns ($FF). 
Current "destructive read" logic is a proxy that might not be 100% accurate for 
all DOS variants.

**Current State**:
- The read path now keeps bit-level internal state (`read_shift_register`,
  `read_bit_phase`) and advances one bit every 4 CPU cycles.
- To preserve DOS 3.3 compatibility, `data_latch` is still only published to the
  CPU at byte boundaries.

**Remaining Gaps**:
- No true P6 state machine yet
- No explicit bit-slip model
- Sync / latch recovery behavior after destructive reads is still a conservative proxy

## 2. 6502 Illegal Opcodes in Official ROMs

**Context**: Some official ROMs or Apple II software use "undocumented" opcodes 
(like $04, $03) for timing or code density.

**Current State**: Core undocumented opcode coverage is now effectively complete,
including NOP/SKB/SKW families, `SLO/RLA/SRE/RRA`, `LAX/SAX/DCP/ISC`,
`ANC/ALR/ARR/AXS`, `KIL/JAM`, and `0xEB` (`SBC #imm` alias).

**Remaining Gaps**:
- Some unstable NMOS-only opcodes (`XAA`, `LAX #imm`, `AHX`, `SHX`, `SHY`, `TAS`)
  are still modeled with deterministic approximations rather than guaranteed
  transistor-accurate behavior.
- Bus-level dummy reads / wrong-page reads have been improved, but full
  verification against external illegal-opcode test suites is still missing.

## 3. Language Card Edge Compatibility

**Context**: The emulator implements a 16KB Language Card model at `$D000-$FFFF`,
including bank switching and write-enable unlock behavior on `$C080-$C08F`.

**What Already Works**:
- Bank 1 / Bank 2 separation in `$D000-$DFFF`
- Shared LC RAM window in `$E000-$FFFF`
- Read-ROM / write-RAM split behavior
- Echo-aware double-read unlock for write-enable switches

**Higher-Risk Remaining Gaps**:
- The exact soft-switch decode matrix may still differ in edge cases from real
  hardware, especially if software relies on unusual `$C08x` access patterns.
- No external Language Card diagnostic or compatibility test suite has been run
  yet, so current confidence comes from targeted unit tests rather than broad
  software coverage.

**Lower-Risk / Deferrable Gaps**:
- Floating-bus return values for LC soft-switch reads are simplified to `0`.
- Very timing-sensitive code that depends on bus noise, analog effects, or
  undocumented switch side effects may still diverge from a real Apple II.
