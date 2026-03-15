# FIXME: Apple II Emulator — Known Issues

## 1. Bit-Level Disk Synchronization

**Context**: Current Disk II emulation works at the byte level (~32 cycles per byte). 
However, some RWTS routines and copy protections rely on the **P6 Sequencer state machine** 
to handle sub-byte bit slips and 10-bit sync patterns ($FF). 
Current "destructive read" logic is a proxy that might not be 100% accurate for 
all DOS variants.

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
