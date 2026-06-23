# Disk Loader Compatibility — Debug Notes

Running log of custom-loader game-disk compatibility work, and how to reproduce
issues headlessly with the `boot_smoke` harness. PicoApple2 (the embedded fork,
byte-level disk core) runs these games fine; apple2emu's experimentally-modified
disk core does not always — these notes track the diffs.

## The headless harness: `boot_smoke`

`cargo run --release --bin boot_smoke` boots a DSK through the native
`Apple2Machine` API (no window) and reports the seek sequence, video mode, motor
status, hot PCs and both text pages. Knobs (env vars):

- `BOOT_DSK=<path>` `BOOT_SECS=<n>` — image and emulated seconds.
- `BOOT_PRESS_BTN=<sec>` — pulse pushbutton 0 + sweep paddles (joystick calibration).
- `BOOT_TYPE='\r'` `BOOT_TYPE_AT=<sec>` — type keys after a delay (waits for the
  strobe to clear between chars). `\r`=Return, `^G`=Ctrl-G.
- `BOOT_DUMP=ADDR,LEN` (hex) — dump a RAM range (inspect loader code).
- `BOOT_TRACE=N` — record up to N run-length-deduped PCs after `BOOT_TYPE_AT`
  (follow control flow after a key is accepted).
- `BOOT_WATCH_PC=ADDR` (hex) — capture the A register every time that PC executes
  (reveals the byte stream a read loop consumes). Prints a histogram + first 48.

It also prints a prologue scan of the track the head ended on (D5AA96 address /
D5AAAD data prologue counts).

## Case: Rescue Raiders (`rescue_raiders.dsk.gz`) — OPEN

**Symptom:** boots to a HIRES title screen that waits for a key; pressing ENTER
loads for a while, then fails and restarts back to the title. Works on PicoApple2.

**Repro:**
```
BOOT_DSK=.../rescue_raiders.dsk.gz BOOT_SECS=60 BOOT_TYPE='\r' BOOT_TYPE_AT=13 \
  cargo run --release --bin boot_smoke
```
(ENTER must land *after* the title settles into the keyboard-wait loop, ~12-13s;
injected earlier it is cleared by the loader's `LDA $C010` and does nothing.)

**Disk:** decompresses to 143488 bytes = 143360 + 128 trailing zero padding
(auto-trimmed). Standard DOS 3.3 sector image.

**Custom loader map (lives in high RAM, not DOS):**
- Title key-wait at `$6100`: `LDA $C010` (clear strobe) / `$6105 LDA $C000` /
  `$6108 BPL $6105` / `$610A RTS` — waits for any key.
- `$7090`: `JSR $7000`; set HIRES/full/page1/graphics; `$709F JSR $6000` (key wait);
  `$70A7 JMP $BFC8` (loader entry).
- `$BFC8`: stash A (part #), set param block at `$BFE8`, `JSR $BCFF` (read), then
  `JMP $4000` (run game) on success.
- `$BCFF`/`$BD00`: read entry. `$BD3B` reads param[2] = track (`$13`=19, counts
  down), `$BD3E JSR $BC9D` (phase-stepped seek), `$BD58 JSR $BC43` (read one
  sector's address field), `$BD5F` checks param[7]=`$FE` volume against `$4C`.
- `$BC43`: find `D5 AA 96`; read 4-and-4 vol/trk/sec/chk into `$4C/$4B/$4A/$49`;
  checksum at `$BC86`; epilogue `DE`/`AA`; `CLC`/`RTS`. (Reads the ADDRESS field
  only; the DATA field is read elsewhere.)

**What is RULED OUT (verified with boot_smoke):**
- Track data is valid: 16 `D5AA96` + 16 `D5AAAD` prologues per track, same as
  MASTER.DSK.
- The read works: at `$BC5C` (the AA compare) A is always `AA`; at `$BC86` the
  address checksum is always `00` (158 samples). Prologue + address field decode
  perfectly. So the read model / byte timing is NOT the problem.
- Read model: swapping the default bit-level shift read for an explicit
  byte-level read (PicoApple2 style) changed the byte stream not at all. Reverted.
- "Preserve rotation phase across a seek" (PicoApple2's track-change fix): no
  effect here. Reverted.
- bit-7 latch filter: our nibblizer only emits bytes >= 0x80, so it never drops
  anything. Non-issue.
- Volume: nibblizer writes vol=254=`$FE`; loader checks `$FE`. Matches.
- 6-and-2 encoding: byte-for-byte IDENTICAL to PicoApple2 (a stale comment in
  nibble.rs claimed a "REVERSED" secondary buffer; the actual code is the
  standard forward layout — comment fixed).

**Only real nibblizer diff was the gap/layout** (now aligned to PicoApple2 in
`nibblize_dsk`): lead-in 64->128, inter-field gap **6->12**, inter-sector gap
27->20, pad each track to the full 6656 bytes. The inter-field gap is the
processing window between the address field and the data prologue. This change
makes Rescue Raiders progress FURTHER (loader runs, motor spins, seeks T16-T20)
but it STILL ultimately fails and returns to the title. MASTER.DSK and The
Goonies still boot/play fine after this change.

**Failure is therefore AFTER the address field** — the data-field read/sequencing,
or per-sector loader logic — and is not fully explained by gaps. NOT yet
root-caused.

**Next step (do NOT keep guessing — pin it):** definitive step trace of the
loader's flow right after a successful `$BC43` address read (`$BD5F` onward, and
the data-field read it eventually calls). Find the exact instruction where it
decides to retry/recalibrate and what it read vs expected. Candidate watches:
the data-prologue search, the data-field checksum, and the sector/volume compares.

## Status of The Goonies — FIXED (for reference)
Boots fully (T0-T23 -> joystick calibration on text page 2 -> button -> T24-T27 ->
HIRES game). Needed only the CPU branch-cycle fix + text page 2 rendering; see
DevLog #24. Repro: `BOOT_PRESS_BTN=5 BOOT_SECS=70`.
