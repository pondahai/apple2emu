# Apple II 磁碟啟動成功之路：技術對話精華錄 (2026-03-12)

本筆記記錄了從「丟失代碼」到「精確 6-and-2 編碼」的還原歷程，解決了磁碟讀取位移與時序同步的核心問題。

## 1. 核心突破：位元級移位暫存器 (Bit-Level Shift Register)
*   **物理模擬**：捨棄了粗略的 32 週期位元組模型，改為每 4 個 CPU 週期移動 1 個位元的精確模擬。
*   **同步機制**：實作了 P6 Sequencer 的位位元移邏輯。當移位暫存器的 Bit 7 變為 1 時，觸發資料就緒（Ready）。這解決了 CPU 輪詢時經常錯過資料窗���的問題。
*   **對準結果**：系統已能穩定自動對準磁區標頭（Prologue: D5 AA 96/AD），並正確跳轉至磁碟執行段（$0801）。

## 2. 物理編碼：黃金對齊公式 (Golden Formula) - 最終確定版
*   **映射分組**：256 bytes 分為 3 組，每組分別取 bits[1:0] 組合到 snib[i%86] 的對應 shift 位置：
    * group 0 (bytes 0–85):   shift 0
    * group 1 (bytes 86–171): shift 2
    * group 2 (bytes 172–255): shift 4
*   **位元互換 (Internal Swap)**：每個 2-bit 值**必須**執行 bit0↔bit1 互換：`bits2_swapped = ((b & 0x01) << 1) | ((b & 0x02) >> 1)`。
    * 原因：RWTS 解碼端在重組原始 byte 時，取 secondary nibble 的方式是 bit1 先、bit0 後，因此 encode 端必須預先對調。
*   **物理順序**：次要緩衝區（前 86 nibbles）以 `snib[85]..snib[0]` 反序寫入；主要緩衝區（後 256 nibbles）以 `sector_data[0]>>2 .. sector_data[255]>>2` 正序寫入。
*   **XOR 鏈累積 (XOR:0)**：`encoded = raw6 ^ last; last = raw6;`（使用原始 6-bit 值作為累積種子，非 encoded 值）。
*   **最終校驗 nibble**：所有 342 個 nibble 發完後，再額外發出 `NIBBLE_WRITE_TABLE[last_val]`，代表最後一個 XOR 殘留值。
*   **磁碟控制器 (read_io)**：**不做毀滅性讀取**。直接 return `self.data_latch`，讓 RWTS 的輪詢迴圈自行判斷 Bit 7 是否就緒。

## 3. Address Field 關鍵修正 ⚠️
*   **sector 欄位必須填物理編號 (`phys_pos`)**，而不是邏輯編號 (`logical_sector`)。
    * RWTS 的磁區搜尋迴圈是比對 Address Field 內的物理磁區號。若填錯，RWTS 將永遠找不到磁區而無限重試。
    * 正確寫法：`let sec: u8 = phys_pos as u8;`

## 4. 磁區間隙 (Gap) 標準值
| 位置 | 用途 | 數量 |
|------|------|------|
| 磁軌開頭 | Pre-gap | 64 x 0xFF |
| Address 與 Data field 之間 | Inter-field gap | 6 x 0xFF |
| 各磁區結尾 | Inter-sector gap | 27 x 0xFF |

## 5. 進度時間線
| 時間 | 里程碑 |
|------|--------|
| 2026-03-10 | 初始 commit，基礎架構建立 |
| 2026-03-11 | 位元級 Disk II 模擬，成功記錄黃金公式 |
| 2026-03-12 上午 | 重新實作，$0800 前 4 bytes 對齊 `01 A5 27 C9` |
| 2026-03-12 下午 | $0800 完整對齊 `01 A5 27 C9 08 D0 1A A5`，DOS 磁頭尋軌成功 |
| 2026-03-12 晚 | nibble.rs 加回正確 bit-swap，$03D0 初始化問題浮現，轉入 CPU 除錯階段 |

## 6. 目前狀態與下一步
*   **磁碟模擬**：`nibble.rs` 的 6-and-2 編碼框架正確。`disk2.rs` 採用 Byte-sync + 不毀滅性讀取。
*   **已知瓶頸**：CPU 在 Stage 2 Boot（$0BB8 附近）跳入 Monitor。原因診斷：
    * `$03D0` 向量未被 DOS 初始化（期望值 `4C 84 9D`，實際為垃圾值）。
    * Track 0 Sector 1–9 或 Track 1/2 的磁區讀取尚未成功，DOS 核心未載入。
    * **最可能根因**：nibble.rs 的 `sec` 欄位填了 `logical_sector` 而非 `phys_pos`，導致 RWTS 無法匹配磁區標頭。
*   **下一步優先行動**：
    1. 修正 `sec: u8 = phys_pos as u8`
    2. 補全 `LAX/SAX/SKB/SKW` 等非法指令
    3. 確認 `$03D0` 是否被正確寫入

---
**最終結論**：「物理模擬太敏感」這個說法是**對的**。Byte-sync + 正確的 nibble 編碼（含 bit-swap + 正確 sec 欄位）是目前最穩定的路線。