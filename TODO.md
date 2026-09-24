# Apple II Emulator - Task List

## 第一階段：完美開機 (已完成)
* [x] **磁碟加載成功** (2026-03-12)
    * 已達成 $0800 完美解碼 `01 A5 27 C9`。
    * 實作 Byte-sync 穩定模型與 $C0EC 存取防護。
* [x] **磁軌定位模擬**
    * 實作精確步進相位模型，支持 DOS 尋軌校準。
* [x] **核心時序修正**
    * 已實作 Page Cross Penalty 與非法 NOP 週期補全。

## 第二階段：功能補強 (進行中)
* [x] **磁碟寫入支援 (Write Mode & State Sequencer)**
    * 實作 Q6/Q7 寫入狀態機與 32-cycle 寫入迴圈時序同步。
    * 修復 Error #4 (Write Protect) 與 Error #8 (I/O Error) 寫入時序問題。
    * 目前僅完成「記憶體內磁軌」寫入；尚未回寫原始 `.dsk` 檔案。
* [ ] **Denibblize 回寫機制**
    * 將記憶體中的磁軌資料 (Nibble) 反解碼回 `.dsk` 格式並存檔。
* [ ] **非法指令與 NMOS 6502 相容性深度補全**
    * `$0BB8` 附近的 DOS Stage 2 / RWTS 崩潰點已由 `SKB/SKW` dummy read 修正解除。
    * opcode coverage、decimal mode、與 undocumented NOP dummy reads 已補齊。
    * 目前剩餘工作聚焦於 unstable undocumented opcodes (`XAA/LAX#imm/AHX/SHX/SHY/TAS`) 的真機近似度，以及更細的 bus/flag/timing 相容性。


## 已知問題備忘
* 磁頭目前的步進暫時使用簡化模型，未來需評估 0.25 軌的細微時序影響。

## 未來實驗構想
* **音訊驅動時鐘 (audio-clock-driven timing)**：目前 (2026-09-25) 主迴圈是用
  wall-clock (`Instant::now()`) 量測真實經過時間來換算該跑幾顆 CPU 週期，這是
  time-based 驅動。另一種老牌模擬器常用的替代設計是反過來，讓音效卡的播放速度
  (audio buffer 實際消耗速度) 當主時鐘來源，CPU 週期預算依音效緩衝區還剩多少來
  反推該執行多少 —— 因為人耳對時間誤差最敏感，用音效當錨點理論上能讓聲音更穩定
  不失真。值得之後找時間實驗比較兩種驅動方式在本專案（尤其是像 Castle
  Wolfenstein 這種頻繁存取磁碟的遊戲）下的實際聽感與畫面流暢度差異。
  相關程式碼位置：`apple2-desktop/src/main.rs` 主迴圈的 `target_cycles` 計算，
  以及 `AudioMixerState`/`sink` 的堆積丟棄機制（見 commit `3a7e85f`）。
