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
* [ ] **Denibblize 回寫機制**
    * 將記憶體中的磁軌資料 (Nibble) 反解碼回 `.dsk` 格式並存檔。
* [ ] **非法指令深度補全**
    * 解決 DOS 核心跳轉至 $B002 後的崩潰問題。


## 已知問題備忘
* 磁頭目前的步進暫時使用簡化模型，未來需評估 0.25 軌的細微時序影響。
