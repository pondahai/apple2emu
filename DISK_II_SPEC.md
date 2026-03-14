# 📊 Apple II Disk II 完整技術規格表

這份文件彙整了 Apple II Disk II 控制器與磁碟系統的硬體規範、時序參數、記憶體佈置及韌體入口點，作為模擬器開發的重要參考。

## 1. I/O 控制寄存器與軟開關 (I/O Control Registers)

| 地址 | 暱稱 | 功能說明 | 操作 | 來源網址 |
|------|------|---------|------|---------|
| $C0n0 | PHASE0 | 步進馬達 Phase 0 | R/W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0n2 | PHASE1 | 步進馬達 Phase 1 | R/W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0n4 | PHASE2 | 步進馬達 Phase 2 | R/W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0n6 | PHASE3 | 步進馬達 Phase 3 | R/W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0n8 | ENABLE | 關閉磁盤驅動器 | W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0n9 | ENABLE | 開啟磁盤驅動器 | W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0nA | SELECT | 選擇驅動器 1 | W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cDA88B95DEDDDE74D810/src/DiskROM.md#L148-L228) |
| $C0nB | SELECT | 選擇驅動器 2 | W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0nC | Q6 | 讀取狀態/寫入數據 | R/W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0nD | WRITE-PROTECT | 檢查寫保護狀態 | R | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0nE | Q7 | 讀取模式控制 | R/W | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |

---

## 2. 時序參數 (Timing Specifications)

| 項目 | 時間 | 說明 | 來源網址 |
|------|------|------|---------|
| 字節時序 | ~32 μs | 在 1MHz 6502 下每字節的時間 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 尋道時序 | ~3 ms | 每磁道的步進馬達速度 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| Track 0 盲尋 | ~200 ms | 最壞情況下的磁頭尋位時間 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 磁盤旋轉 | 200 ms | 完整一圈磁盤旋轉時間 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 馬達啟動延遲 | ~1 s | 磁盤驅動馬達達到全速所需時間 | [pcornier/iigs_simulation](https://github.com/pcornier/iigs_simulation/blob/08ca4dd56180136b4fd61fed5be53484df1623fb/doc/disk5.25.txt#L35-L106) |

---

## 3. 記憶體佈局 (Memory Layout)

| 地址範圍 | 大小 | 名稱 | 用途 | 來源網址 |
|---------|------|------|------|---------|
| $0100-$01FF | 256 B | STACK | 6502 棧 (系統範圍) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $0200-$02FF | 256 B | (保留) | 一般用途 RAM | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $0300-$0355 | 86 B | TWOS_BUFFER | 6+2 解碼的 2 位塊緩衝 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $0356-$03D5 | 128 B | CONV_TAB | 6+2 轉換解碼器表 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $03D6-$07FF | ~1.5K | (可用) | 一般用途 RAM | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $0800-$0BFF | 1K | BOOT1 | 次級啟動加載程序代碼 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $C0n0-$C0nF | - | 控制器 I/O | 馬達和磁盤控制寄存器 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |

---

## 4. 零頁變數 (Zero-Page Variables)

| 地址 | 名稱 | 用途 | 來源網址 |
|------|------|------|---------|
| $26-$27 | data_ptr | 指向 BOOT1 數據緩衝區位置的指針 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $2B | slot_index | 插槽號 << 4 (用於插槽相對控制器 I/O 尋址) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $3C | bits | 6+2 解碼期間位操作的臨時存儲 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $3D | sector | 正在讀取的扇區號 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $40 | found_track | 尋道期間找到的磁道 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $41 | track | 要讀取的磁道 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |

---

## 5. 插槽配置 (Slot Configuration)

| 插槽 | ROM 位置 | I/O 位置 | 描述 | 來源網址 |
|------|---------|---------|------|---------|
| Slot 5 | $C500-$C5FF | $C580-$C58F | 可選 Disk II 位置 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| Slot 6 | $C600-$C6FF | $C680-$C68F | **標準 Disk II 位置** | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| Slot 7 | $C700-$C7FF | $C780-$C78F | 可選 Disk II 位置 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |

---

## 6. 韌體入口點 (Firmware Entry Points)

| 地址 | 名稱 | 功能說明 | 來源網址 |
|------|------|---------|---------|
| $C600 (Slot 6) | ENTRY | 主啟動入口點，初始化控制器並加載 BOOT1 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $Cn01 | 相對跳轉 | 相對跳轉到實際 $C600 入口點 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $FCA8 | MON_WAIT | 時序關鍵操作的延遲例程 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| $FF58 | MON_IORTS | 系統識別/插槽檢測 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |

---

## 7. 讀寫操作序列 (Read/Write Operation Sequence)

| 操作類型 | 步驟 | 詳細說明 | 來源網址 |
|---------|------|---------|---------|
| **寫保護檢查** | 1 | `LDA $C08D,X` | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| | 2 | `LDA $C08E,X` - 重置狀態序列器 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| | 3 | `BMI WPROTECT` - 如果受保護則分支 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L148-L228) |
| **讀取模式** | 1 | `LDA $C08E,X` - 啟用讀取模式 | [pcornier/iigs_simulation](https://github.com/pcornier/iigs_simulation/blob/08ca4dd56180136b4fd61fed5be53484df1623fb/doc/disk5.25.txt#L35-L106) |
| | 2 | 等待 $C08C 高位打開 | [pcornier/iigs_simulation](https://github.com/pcornier/iigs_simulation/blob/08ca4dd56180136b4fd61fed5be53484df1623fb/doc/disk5.25.txt#L35-L106) |
| | 3 | 從 $C08C 讀取數據字節 | [pcornier/iigs_simulation](https://github.com/pcornier/iigs_simulation/blob/08ca4dd56180136b4fd61fed5be53484df1623fb/doc/disk5.25.txt#L35-L106) |
| **寫入模式** | 1 | 硬件每 4 個週期寫入 1 位 | [pcornier/iigs_simulation](https://github.com/pcornier/iigs_simulation/blob/08ca4dd56180136b4fd61fed5be53484df1623fb/doc/disk5.25.txt#L35-L106) |
| | 2 | 程序必須以精確的 32 個週期間隔提供字節 | [pcornier/iigs_simulation](https://github.com/pcornier/iigs_simulation/blob/08ca4dd56180136b4fd61fed5be53484df1623fb/doc/disk5.25.txt#L35-L106) |
| | 3 | 控制器不會告訴何時寫入數據 | [pcornier/iigs_simulation](https://github.com/pcornier/iigs_simulation/blob/08ca4dd56180136b4fd61fed5be53484df1623fb/doc/disk5.25.txt#L35-L106) |

---

## 8. 6502 優化技術 (6502 Optimization Techniques)

| 技術 | 用途說明 | 應用場景 | 來源網址 |
|------|---------|---------|---------|
| 自修改代碼 | 修改循環內的分支目標以提高效率 | 高速磁盤讀取循環 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 索引尋址 | 使用 `LDA addr,X` 進行插槽相對硬件訪問 | 控制器 I/O 訪問 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 間接尋址 | `LDA (data_ptr),Y` 用於緩衝區訪問 | 數據緩衝區操作 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 分支優化 | 仔細放置分支以避免頁邊界交叉 | 性能優化 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |

---

## 9. 錯誤處理策略 (Error Handling)

| 處理類型 | 行為說明 | 後果 | 來源網址 |
|---------|---------|------|---------|
| 無限重試 | 如果未找到扇區，持續搜索同一磁道 | 可能導致無窮循環 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 無超時限制 | 磁盤出現錯誤時會無限期掛起 | 系統無響應 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |
| 靜默失敗 | 失敗時無任何報告 | 用戶看到黑屏 | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/src/DiskROM.md#L428-L510) |

---

## 10. 軟開關控制 (Display & Memory Control Soft Switches)

| 地址 | 功能 | 操作 | 狀態檢查 | 來源網址 |
|------|------|------|---------|---------|
| $C000 | 80STORE OFF | W | $C018 (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |
| $C001 | 80STORE ON | W | $C018 (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |
| $C002 | RDMAINRAM | R/W | $C013 (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |
| $C003 | RDCARDRAM | R/W | $C013 (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |
| $C004 | WRMAINRAM | R/W | $C014 (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |
| $C005 | WRCARDRAM | R/W | $C014 (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |
| $C054 | PAGE1 | R | $C01C (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |
| $C055 | PAGE2 | R | $C01C (R7) | [tmcintos/OpenA2FirmwareSpecification](https://github.com/tmcintos/OpenA2FirmwareSpecification/blob/dc4a7e4166c35c2488f4cda88b95deddde74d810/OpenA2FirmwareSpecification.md#L1110-L1190) |

---

## 📚 參考資源匯總

- **OpenA2FirmwareSpecification**: [GitHub 連結](https://github.com/tmcintos/OpenA2FirmwareSpecification)
- **pcornier/iigs_simulation**: [GitHub 連結](https://github.com/pcornier/iigs_simulation)
- **cmosher01/Apple-II-Source**: 包含系統軟體與 DOS 源代碼參考。
