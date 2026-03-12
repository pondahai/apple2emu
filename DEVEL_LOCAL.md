# Apple II Emulator - Local Development Notes (DO NOT COMMIT)

## 執行環境
- **OS**: Windows 10/11 (win32)
- **Project Root**: `C:\Users\Dell\Documents\GitHub\apple2emu`
- **ROMs Directory**: `roms/` (包含 APPLE2PLUS.ROM, DISK2.ROM, MASTER.DSK)

## 常用指令
### 1. 執行模擬器 (主程式)
```powershell
taskkill /F /IM apple2-desktop.exe /T ; cargo run --bin apple2-desktop --quiet
```

### 2. 驗證工具
```powershell
cargo run --bin verify_nibble --quiet
```

## 目前狀態 (2026-03-12) - **重大突破**
- **Disk II (已成功)**: 
    - **解碼正確**: 記憶體 `$0800` 成功讀出 `01 A5 27 C9 08 D0 1A A5` (DOS 3.3 啟動磁區)。
    - **黃金公式**: 採用 Byte-sync (32 cycles/byte) + Bit-Swap + SOff:10 + XOR:0 (累積原始值)。
    - **硬體保護**: 實作了 `$C0EC` 的 I/O 存取保護，避免誤觸寫入模式。
    - **尋軌成功**: 觀察到 Track 0 <-> 1 的物理尋軌動作，代表 DOS 核心已接管磁碟機。

- **CPU (目前瓶頸)**:
    - **崩潰點**: 執行進入 Stage 2 Boot 後，在 **`$0BB8`** 附近跳入 Monitor (`*`)。
    - **初步診斷**: 可能與未實作的非法指令 (Illegal Opcodes) 副作用或旗標 (Flags) 的細微差異有關。
    - **待辦**: 補全 `LAX/SAX/DCP/ISC` 並確保 NOP 變體 (SKB/SKW) 執行真實的記憶體讀取。

## 已知問題備忘
- 磁軌尋軌已採用相位拉力模型，對準精度大幅提升。
- `$0100` (Stack) 在崩潰後顯示全 `00`，疑似觸發了 `BRK` ($00)。
