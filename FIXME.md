# FIXME: Apple II Emulator — Known Issues

## 1. Bit-Level Disk Synchronization

**Context**: Current Disk II emulation works at the byte level (~32 cycles per byte). 
However, some RWTS routines and copy protections rely on the **P6 Sequencer state machine** 
to handle sub-byte bit slips and 10-bit sync patterns ($FF). 
Current "destructive read" logic is a proxy that might not be 100% accurate for 
all DOS variants.

## 2. 6502 Illegal Opcodes in Official ROMs

**Context**: Some official ROMs or Apple II software use "undocumented" opcodes 
(like $04, $03) for timing or code density. These currently trigger debug prints 
or unexpected behavior.
