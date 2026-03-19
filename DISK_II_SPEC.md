# Apple II Disk II Low-Level Technical Specifications

這是一份結合了極簡硬體設計與極高複雜軟體編碼的低階技術資料，是開發高相容性 Apple II 模擬器的核心參考。

## 1. I/O 暫存器位址 (Memory Mapped I/O)

Disk II 控制卡通常插在 Slot 6，其基底位址為 `$C0E0` (Slot 6) 或一般項 `$C0n0` ($n = \text{Slot} + 8$)。

| 位址 (十六進位) | 功能描述 |
| :--- | :--- |
| **$C080 / $C081** | 相位 0 (Phase 0) 關閉 / 開啟 (控制步進馬達) |
| **$C082 / $C083** | 相位 1 (Phase 1) 關閉 / 開啟 |
| **$C084 / $C085** | 相位 2 (Phase 2) 關閉 / 開啟 |
| **$C086 / $C087** | 相位 3 (Phase 3) 關閉 / 開啟 |
| **$C088 / $C089** | 磁碟馬達 (Motor) 關閉 / 開啟 |
| **$C08A / $C08B** | 選擇磁碟機 1 / 磁碟機 2 |
| **$C08C / $C08D** | 讀取移位暫存器 (Shift) / 載入暫存器 (Load) |
| **$C08E / $C08F** | 切換至讀取模式 (Read) / 寫入模式 (Write) |

*註：讀寫操作高度依賴精確的 CPU 時序，通常以 32 或 40 個時脈週期為一個位元組的處理單位。*

---

## 2. 硬體邏輯與狀態機 (The Sequencer)

Apple II 磁碟控制器的核心是一個由 **P6 PROM (256x8)** 驅動的狀態機 (State Machine)。

*   **P6 PROM**: 根據目前的狀態（State）、資料匯流排輸入以及寫入保護訊號，決定 74LS299 移位暫存器的動作（Shift 或 Load）。
*   **作用**: 負責將磁碟上的脈衝序列轉化為 CPU 可讀取的位元組。理解這 256 Byte 的狀態表是編寫精確模擬器的關鍵。

---

## 3. 編碼與磁區格式 (Encoding & Format)

Disk II 使用 Wozniak 自創的 **GCR (Group Code Recording)**。

*   **6-and-2 Encoding**: DOS 3.3 標準。將 6 位元的資料映射為 8 位元的磁碟位元組，確保不會出現過多連續的「0」。
*   **Nibblizing**: 資料寫入前會拆解並轉換成 342 個 Nibbles。
*   **Sync Bytes**: 通常是 `$FF`。為了對齊位元組邊界，磁碟會寫入超過 8 位元長度的特殊 `$FF` (Auto-sync bytes)。

---

## 4. 低階磁區結構 (Low-level Track Layout)

每一條軌道由多個磁區組成，包含：

1.  **Address Field (位址欄位)**: `$D5 AA 96` (起始標記) + Volume/Track/Sector/Checksum + `$DE AA EB` (結束標記)。
2.  **Data Field (資料欄位)**: `$D5 AA AD` (起始標記) + 342 個 GCR 寫碼後的位元組 + Checksum + `$DE AA EB` (結束標記)。

---

## 5. 未公開與進階技巧 (Undocumented & Advanced)

*   **Half-tracking (半軌存取)**: 步進馬達可移動到軌道中間（如 Track 1.5），常用於防拷技術。
*   **Quarter-tracking**: 透過精確控制多個相位磁鐵，達成 1/4 軌道的微移。
*   **Bit-slip (位元滑動)**: 故意寫入不符合規範的位元序列，迫使硬體失去同步以偵測原始磁碟。
*   **Hidden Tracks**: 物理支援到 40 軌 (Track 0-39)，但標準 DOS 僅用到 34 軌。
*   **Motor Timing**: 馬達啟動需約 150ms 穩定，關閉後有約 1s 慣性轉動時間。

---

## 6. 核心參考文獻清單 (必讀資料)

1.  **《Understanding the Apple II》 (Jim Sather)**: 第九章 "Disk II Controller" 是最詳盡的硬體分析，包含 P6 PROM 狀態圖。
2.  **《Beneath Apple DOS》 (Don Worth & Pieter Lechner)**: 研究軟體層級（RWTS、磁區結構、防拷技術）的聖經。
3.  **《The Apple II Circuit Description》 (Winston Gayler)**: 提供極其精確的硬體電路時序圖。
4.  **《Beneath Apple ProDOS》**: 研究 ProDOS 區塊存取邏輯必看。
