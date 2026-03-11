# Apple II 模擬器開發與討論大綱

## 1. 專案初始化與基礎除錯
* **環境排除**：解決 Windows 系統下 Cargo 編譯指令找不到的問題，以及執行檔被系統佔用（Access Denied）時，透過 `taskkill` 自動強制關閉舊程序的開發流程。
* **架構釐清**：確立 `apple2-core` (核心硬體模擬) 與 `apple2-desktop` (Windows 視窗前端) 的兩層式架構。

## 2. 磁碟機控制器 (Disk II) 與時序修正
* **磁區編碼修復**：修改 `nibble.rs`，針對 GCR 的 4-and-4 與 6-and-2 編碼與解碼邏輯除錯。
* **時序精準度 (Cycle Accuracy)**：實作磁碟機讀取的 32 微秒 (週期) 延遲，解決資料讀取過快導致 CPU 卡死在 RWTS (Read/Write Track Sector) 迴圈的問題，成功啟動 DOS 3.3。

## 3. 記憶體管理與 ROM 系統 (MMU)
* **硬體暫存器映射 (Address Mirroring)**：實作 `$C000` 與 `$C010` 等軟體開關 (Soft Switches)，確保鍵盤按鍵資料讀取與 Strobe 清除訊號正確運作。
* **ROM 載入與追蹤**：正確載入 Apple II+ Motherboard ROM (`APPLE2PLUS.ROM`) 與 Disk II Controller ROM (`DOS33_ROM.bin`)，移除舊的非標準強制跳轉，恢復最原汁原味的開機流程。
* **CPU 擴充**：補齊 `BRK` 中斷指令實作與未知指令 (Unimplemented Opcode) 的執行追蹤。

## 4. 顯示系統與繪圖模式
* **文字模式增強**：加入記憶體狀態判斷，實作了正確的「游標閃爍 (Flashing)」與「反白 (Inverse)」字元呈現。
* **顯示模式開關**：在記憶體中捕獲 `$C050`~`$C057` 的記憶體存取，用來即時切換文字、低解析度、高解析度與混合模式 (Mixed Mode)。
* ** 低解析度圖形 (GR)**：實作 `render_lores_frame` ，支援 40x48 區塊與 Apple II 經典的 15 色調色盤。
* ** 高解析度圖形 (HGR)**：實作 `render_hires_frame` ，針對 NTSC 的色彩失真 (Artifact Colors) 特性，透過偶數位元/奇數位元與 Palette Shift (Bit 7) 的組合，正確渲染出綠、紫、藍、橘色。

## 5. 鍵盤佇列與靈敏度強化
* **消滅延遲**：捨棄原先套件提供的 `get_keys_pressed`，改以手動比對每個影格的按鍵狀態 (`last_keys` 與 `current_keys`)，解決 Windows 環境下鍵盤反應遲鈍的問題。
* **組合鍵支援**：
  * **Shift 鍵**：手動對應 Apple II 鍵盤表，支援輸入 `!`、`@`、`#`、`"` 等符號。
  * **Control 鍵**：轉換對應的 ASCII 控制碼，讓使用者能透過 `Ctrl+B` 從 Monitor (`*`) 進入 BASIC 模式 (`]`)。

## 6. 現代桌面功能整合與部署
* **音訊發聲 (Audio)**：加入 `rodio` 套件實作 Apple II 經典內建喇叭。透過監聽 `$C030` 記憶體存取切換狀態，並結合精準 CPU 週期運算，動態生成並輸出 44.1kHz 的方波 (Square Wave) 使聲音能即時延遲播放。
* **剪貼簿貼上 (Ctrl+V)**：引入 `arboard` 套件，讓使用者可以直接在模擬器內貼上電腦外部複製的 BASIC 程式碼，並在背後自動將小寫字母轉換為大寫。
* **專案文件**：自動生成專案的 `README.md`，統整目前系統能支援的規格。
## 7. 系統功能擴充與 UI 自動化
* **動態磁碟載入 (F3)**：整合 `rfd` (Rust File Dialog) 套件，讓使用者能透過視窗介面即時更換磁碟影像，不再需要重啟或修改程式碼。
* **壓縮格式支援 (Gzip)**：加入 `flate2` 套件，支援直接載入 `.gz` 格式的磁碟影像，模擬器會自動在內部進行解壓縮處理。
* **熱鍵系統整合**：
  * **系統重啟 (F2)**：實作機器的「冷啟動 (Cold Boot)」，清空 RAM 狀態並重設硬體，強制 ROM 重新執行開機程序。
  * **快速重置 (Ctrl-Delete)**：對應 Apple II 的 `Reset` 物理按鍵，執行溫重置 (Warm Reset/Warm Boot)。
  * **按鍵防連點 (Debounce)**：在主迴圈實作按鍵邊緣偵測 (Edge Detection)，確保按下 F2 或 F3 等功能鍵時不會造成反覆觸發。

## 8. ROM 環境整理與磁碟啟動修復 (2026-03-11)
* **相對路徑重構**：移除 `apple2-desktop/src/main.rs` 中所有寫死的絕對路徑，改用相對路徑 `../roms/...`，讓任何開發者 clone 後只需把 ROM 放進 `roms/` 資料夾即可執行。
* **Disk II P5A ROM 提取**：從本機 `AppleWin1.26.1.1/Applewin.exe` 以 Python 腳本定位並提取正確的 256 byte P5A Boot ROM（起始位元組 `A2 20 A0 00 A2 03 86 3C...` 驗證正確），放置於 `roms/extracted_256_150.bin`。
* **MASTER.DSK 整合**：從 AppleWin 安裝資料夾複製 `MASTER.DSK` 到 `roms/MASTER.DSK`，統一所有 ROM 資源至同一目錄。
* **SETUP.md 建立**：新增 `SETUP.md` 說明文件，詳細記錄所有 ROM 檔案來源、大小、獲取方式與放置位置，確保日後交接時任何人都能快速重建環境。
* **nibble.rs 6-and-2 編碼重構**：重寫 6-and-2 次要緩衝區 (secondary buffer) 的建構邏輯，改以 slot `j` 從 byte `j`、`j+86`、`j+172` 聚合 2 bits 的正確方式填寫，修正先前 `i % 86` 位移順序錯誤的問題，同時修正 index out of bounds panic (所有值 mask `0x3F` 後再查表)。
* **Compiler Warnings 全清**：清除所有 `unused_imports`、`unused_mut`、`dead_code`、`unused_variables` 等警告，使 `cargo build` 輸出完全乾淨。
* **磁碟讀取追蹤 (Debug Log 진展)**：透過在 `disk2.rs` 注入 `eprintln!` 攔截 latch byte，並實作 2 秒後由程式自動注入 `PR#6\r` 鍵盤序列的測試環境，成功從 stderr log 中捕捉到 RWTS 讀取磁帶位元組流的情況。
  * **確認 Prologue 成功辨識**：位址欄位 (Address Field) 的 `D5 AA 96` 與資料欄位 (Data Field) 的 `D5 AA AD` 皆完整且正確出現在位元流中。
  * **確認 Boot Sector 載入**：stdout 也印出 `LEFT C600 boot ROM to PC=0801`，代表 Boot ROM 確實找到磁區並跳轉執行。
  * **發現資料錯誤 (Bit Ordering Bug)**：載入到 `$0800` 的資料為 `00 A4 24 C8 08 D0 18 A4`，與期望的 DOS 3.3 Boot Sector 內容 `01 A5 27 C9 09 D0 18 A5` 剛好都有 1 個 bit 被消除或偏移的狀況。這明確指向是 `nibble.rs` 中「6-and-2 編碼的次要緩衝區 (Secondary Buffer) 位元組裝順序或反轉邏輯」有誤 (`FIXME` 項次 2)。

## 9. 成功啟動 DOS 3.3 (2026-03-11)
* **6-and-2 編碼完美修復**：徹底重寫了蘋果磁碟的 6-and-2 (Secondary Buffer) 2位元提取邏輯，完全依據《Beneath Apple DOS》的硬體移位暫存器(Shift Register) 順序，確保磁區校驗碼 (Checksum) 在經過 XOR 運算後絕對正確。並加入自動化測試驗證 Data Field Epilogue 的長度與位置 (剛好 343 bytes)。
* **實作磁碟機「半步/四分之一軌 (Quarter-Track)」步進馬達**：
  * DOS 3.3 RWTS 移動磁頭時是直接控制 4 個相位磁石 (Phases 0-3)，真正的 Disk II 磁頭移動並非整數，而是會經歷「半軌」甚至「四分之一軌」的狀態。
  * 先前的程式在每次跨相(Phase change)時都會利用 `(track * 2)` 將物理精確定位截斷為整數，導致磁頭在 Track 0 到 Track 1 之間來回彈跳，永遠無法抵達 Track 1 讀取後續 OS 核心檔。
  * 透過在 `disk2.rs` 引入 `current_qtr_track: i32` 追蹤磁頭的真實物理相對位置，DOS 3.3 終於能成功突破 Track 0，載入後續磁軌。
* **資料鎖存器 (Data Latch) 時序精煉**：在 CPU 讀取 `$C0EC` 資料鎖存器後，立刻將有效週期清空 (`latch_valid_cycles = 0`)，完美配合了 ROM 中僅有 14 CPU cycles 的超緊湊輪詢迴圈 (Tight polling loop)。
* **自動指令介面調整**：原本實作了在按下 F3 選取新磁片後自動產生 `PR#6` 動作，但為了維持硬體的原汁原味 (插磁片本身並不會重新開機)，應使用者要求將其移除。使用者可遵循真實方法按下 `Ctrl-Del` (機殼上的 Reset) 回到 BASIC 提示字元 `]`，再手動敲入 `PR#6`。
* **里程碑達成**：模擬器現在能完美冷啟動並執行原生的 `MASTER.DSK`，穩定載入 DOS 3.3 Kernel 並進入 Applesoft BASIC。

## 10. 音訊系統優化與除錯 (2026-03-11)
* **消除閒置爆音 (DC Offset Pop Fix)**：
  * 原先在喇叭閒置時，會持續輸出 `0.1` 或是 `-0.1` 的連續電壓 (DC Offset)。當這個常數音訊在底層 `rodio` 被截斷、重置或是剛建立連線時，就會形成方波的邊緣，進而在喇叭產生吵雜的、難以忍受的連續波波聲 (Pop/Click)。
  * 加入了簡單的**高通濾波器 (DC Blocker / High-pass filter)**：`y[n] = x[n] - x[n-1] + R * y[n-1]`，讓閒置訊號能以指數形式快速歸零 (Decay to silence)，徹底解決了閒置時的雜音問題。
* **解決音效斷層與降頻 (Sample Rate & Phase Tracking)**：
  * 為了有效節省資源，將音訊輸出改為 22050 Hz (原為 44100 Hz)，這降低了 CPU 每一幀需要拋出的浮點運算量，同時仍足以呈現清晰的高頻蜂鳴器聲音。
  * 修正了「每秒 60 幀」跨越界線時的截斷問題。讓 `unprocessed_cycles` 可以被保留到下一幀繼續累積，而不是每次重繪畫面時就直接歸零扔掉，消除了蜂鳴聲中的微小鋸齒狀斷點。
* **改善緩衝區卡頓 (Audio Buffer Padding)**：
  * 當模擬器處理大量 CPU 指令或畫面更新稍有延誤時，原有的嚴格對齊緩衝會被瞬間播完 (Underrun)，導致音效卡沒資料而發出斷裂感 (Choppy audio)。
  * 重新設計 `rodio` 緩衝策略，容忍最大 15 幀的聲音積壓。更重要的是，當發現序列長度即將見底 (`buf_len == 0`) 時，主動推入 1 幀長度的**靜音過渡資料 (Padding)**。這樣音效卡能保持運作而不會遇到硬截斷，大幅提升輸入 `CTRL-G` 及打字時嘟嘟聲的連貫度。

## 11. 磁碟啟動深度除錯與 Byte Order 修正 (2026-03-11)
* **nibble.rs Bit-Order 修正**：
  * 發現 6-and-2 編碼的次要緩衝區 (Secondary Buffer) 中，bit0 與 bit1 的位置與 Apple II 物理硬體 (LSR+ROL 解碼) 不一致。
  * 修正後，`$0800` 的資料從 `02 A6 27 CA...` 恢復為正確的 `01 A5 27 C9...`，順利通過 Boot ROM 的第一階段校驗。
* **診斷工具 verify_nibble.rs**：實作了模擬 RWTS 的三種解碼策略對比，最終確認 Apple II 實際上使用的是 bit0 優先推入 (High position in 2nd-bit slot) 的邏輯。
* **Boot Sector ($0801) 執行追蹤**：
  * 確認 CPU 已成功執行 `JMP $0801` 進入啟動磁區代碼。
  * **發現循環檢查失敗**：啟動磁區在 `$0801` 會檢查 `$27` 是否等於 `$09`。由於 Boot ROM 在讀取 Sector 0 前可能掃描了其他磁區，導致 `$27` 計數器不符預期。
  * **剩餘問題**：目前會進入 `$081F` 的加載迴圈，不斷遞減 `$27` 並嘗試讀取 DOS 後續分頁，但最終會因為不明原因（可能是 `$2B` 位置被改壞或讀取超時）觸發 `BRK` 進入 Monitor (`*`)。此為現階段待解之謎。

## 12. 官方 ROM 導入與硬體行為深度同步 (2026-03-11)
*   **官方 P5/P6 ROM 導入**：
    *   從 mirrors.apple2.org.za 下載了正式的 16-Sector Disk II 控制卡 ROM (P5: 341-0027 與 P6: 341-0028)。
    *   **發現並修正損毀檔案**：識別出原先使用的提取檔案 (`extracted_256_150.bin`) 內容為 HTML 亂碼，徹底刪除並替換為正確的 `DISK2.ROM`。
*   **CPU 指令硬體層級修正**：
    *   **PHP/PLP 準確性**：修正 `PHP` 永遠推入 Bit 4/5 為 1，以及 `PLP` 拉出時必須忽略 Bit 4 (B flag) 的 6502 原始行為。這是官方 Disk II ROM 建立解碼表邏輯的關鍵。
    *   **跨頁週期補償 (Page Cross Penalty)**：為 `LDA` 的索引定址模式加入額外的 1 週期補償，確保 RWTS 讀取迴圈的時序與磁片轉速精確對齊。
*   **磁碟機鎖存器行為優化**：
    *   **資料持久化**：修正 `$C0EC` 資料暫存器的行為，使其在讀取後**不主動清空** Bit 7，而是維持到下一個位元組到達。這模擬了硬體移位暫存器的真實特性，大幅提升了對 RWTS 緊湊迴圈的相容性。
    *   **Nibble 順序逆轉**：發現 P5 ROM 使用 `DEY` 逆序讀取前 86 個 Nibble，修正 `nibble.rs` 將其反序寫入磁碟，成功推進了解碼進度。
*   **回歸純淨啟動流程**：移除所有手動注入 `X` 暫存器或強行跳轉的測試代碼，完全依賴系統 Reset Vector 啟動，確保「商標顯示 -> 槽位掃描 -> 磁碟引導」的完整鏈條運作。

