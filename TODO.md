# TODO: Apple II Emulator — Next Steps

## 1. High-Accuracy Disk II Emulation

- [ ] Implement bit-level state machine using the **P6 Sequencer ROM (341-0028)**.
- [ ] Implement support for 10-bit Self-Sync bytes ($FF) in `nibble.rs`.
- [ ] Refine half-track and quarter-track head positioning logic.

## 2. CPU Core Refinement

- [ ] Fully implement and verify **Page Cross Cycle Penalties** for all indexed instructions.
- [ ] Add unit tests for `ADC`/`SBC` decimal mode and flag correctness.
- [ ] Support common "illegal" opcodes ($04, $0C, $80 BRA) required by some Disk II firmwares.

---

## 3. Advanced Features

- [ ] Apple //e extended memory support (80-column card, AUX RAM).
- [ ] Joystick / game paddle emulation (PDL0/PDL1 via mouse position).
- [ ] Save/restore machine state (snapshot).
- [ ] Web target via WASM (using `apple2-core` no_std library).
