# Apple II Emulator - Tomorrow's Action Plan

## 第一階段：恢復「黃金開機版」(優先級：最高)
* [ ] **核心時序修正**：
    * 修改 `instructions.rs`：`PHP` 壓入 0x30，`PLP` 忽略 Bit 4。
    * 修改 `cpu.rs`：`step` 必須回傳 `cycles + extra` (Page Cross)。
* [ ] **磁碟編碼修正**：
    * 修改 `nibble.rs`：針對 P5 ROM 的 `DEY` 特性，將 Secondary Buffer (前 86 bytes) 物理反序寫入。
    * 確保 XOR 鏈條邏輯：`Disk[i] = Data[i] ^ Data[i-1]`。
* [ ] **連動與映射**：
    * 修改 `machine.rs`：補回 `disk2.tick(cycles)`。
    * 修改 `memory.rs`：確保 `$C600` 映射到 Slot 6 ROM。

## 第二階段：功能補強 (開機成功後再動)
* [ ] **鍵盤符號全對應**：將今天的 `Shift` 映射表完整遷入。
* [ ] **非法指令安全評估**：只有在 RWTS 穩定後，才考慮加入 `4B` 等指令，且必須確保不是亂碼誤報。
* [ ] **寫入功能重啟**：從 `DevLog #12` 的持久化 Latch 基礎上，重新設計 `write_io`。

## 硬體參數備忘 (不可變動)
* 磁碟轉速：32 CPU Cycles / Byte。
* 磁頭步進：支援 0.25 軌 (current_qtr_track)。
* 鎖存器行為：`$C0EC` 讀取後不清空 Bit 7。
