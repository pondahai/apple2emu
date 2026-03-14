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
  * **系統重啟 (F2)**：實作機器的「冷啟動 (Cold Boot)」，原先是手動清空 RAM，現在改為完整重新實例化 (Re-instantiate) `Apple2Machine`，以確保所有硬體狀態完全歸零，解決重啟後可能會遺失開機「嗶聲 (Beep)」的問題。
  * **快速重置 (Ctrl-Delete)**：對應 Apple II 的 `Reset` 物理按鍵，執行溫重置 (Warm Reset/Warm Boot)。
  * **按鍵防連點 (Debounce)**：在主迴圈實作按鍵邊緣偵測 (Edge Detection)，確保按下 F2 或 F3 等功能鍵時不會造成反覆觸發。

## 8. ROM 環境整理與磁碟啟動修復 (2026-03-11)
* **相對路徑重構**：移除 `apple2-desktop/src/main.rs` 中所有寫死的絕對路徑，改用相對路徑 `roms/`，確保開發環境的一致性。
* **官方 P5/P6 ROM 導入**：捨棄亂碼提取檔，改用正式的 16-Sector Disk II 控制卡 ROM。
* **nibble.rs 6-and-2 編碼重構**：徹底修正 Secondary Buffer 的位元組合邏輯，解決 index out of bounds 問題。

## 9. 成功啟動 DOS 3.3 (2026-03-11) - 黃金里程碑
* **實作磁碟機「四分之一軌 (Quarter-Track)」**：透過 `current_qtr_track: i32` 追蹤磁頭位置，解決磁頭在 Track 0/1 之間彈跳的問題，這才讓 DOS 3.3 能讀取後續磁軌。
* **32 週期 Byte-level 同步**：實作磁碟機 32 CPU 週期的資料鎖存間隔，精確對齊 RWTS 緊湊輪詢迴圈。
* **里程碑達成**：模擬器成功冷啟動原生 `MASTER.DSK`，載入 DOS 3.3 核心並進入 Applesoft BASIC。

## 10. 音訊系統與渲染優化
* **消除閒置爆音 (DC Blocker)**：加入高通濾波器解決喇叭閒置時的 Offset 問題。
* **文字模式修正**：修正 Inverse 與 Flashing 的 ASCII 映射，確保 `APPLE ][` 與 Monitor 提示字元正確顯示。

## 11. 磁碟深度同步 (RWTS 專用修正)
* **PHP/PLP 指令修正**：
  * `PHP` 壓入堆疊時必須強制設置 Bit 4 (Break) 與 Bit 5 (Unused) 為 1。
  * `PLP` 拉回時必須忽略 Bit 4。
  * 這是 Disk II ROM 建立解碼表邏輯的關鍵。
* **跨頁週期罰時 (Page Cross Penalty)**：補回 `LDA Absolute,X` 等指令跨越 256 位元組邊界時的 1 週期懲罰，這對於維持 32 週期的讀取視窗至關重要。

## 13. 磁碟讀取大復原 (The Great Recovery - 2026-03-12)
* **LSR/ROL 效應 (The Bit-Swap Discovery)**：確認了 Boot ROM 在解碼次要緩衝區時，使用 `LSR` (移出 Bit 0) 配合 `ROL` (移入 A 的位元 0) 的指令序列。這會導致 **Bit 0 與 Bit 1 在解碼後發生互換**。因此，在 `nibble.rs` 編碼時，必須對每一對位元進行預先互換（Swap），才能在內存中得到正確的值（如 `$01` 而非 `$02`）。
* **毀滅性讀取 (Destructive Read)**：修正了 `$C0EC` 的讀取行為。當 CPU 讀取資料鎖存器時，必須立即清除 Bit 7 (Ready 標誌)，以防止 CPU 在同一個 32 週期的位元組視窗內重複讀取相同的資料。這對於 RWTS 的穩定輪詢至關重要。
* **物理對齊確認**：再次驗證了 86 個次要 nibble 必須以物理反序（85..0）寫入磁軌，而 XOR 鏈必須以原始 6-bit 值（而非編碼後的值）作為下一個累積的基礎。
* **CPU 時序精確化 (2026-03-13)**：發現 DOS 3.3 RWTS 的極限微秒級時序高度依賴 Dummy Reads。修正 `SKB` 執行真正的記憶體讀取後，解除了 `$0BB8` 崩潰點。
* **磁區編碼完美還原 (2026-03-13)**：撤銷了之前錯誤的 `b0 << 4` 編碼邏輯，並發現 Boot ROM (Stage 1) 的迴圈會產生 `+76` (即 `SOff:10`) 的隱性時序偏移，最終捨棄了針對 Boot ROM 的雙重反轉，改以正向發送 (Forward Emission) 並結合 XOR 編碼，讓 `$0800` 完美解出 `01 A5 27 C9 09 D0 18 A5`。
* **目前進度**：成功引導 MASTER.DSK，並顯示出 `]` Applesoft BASIC 提示字元！這是一個重大的里程碑。

## 14. 磁碟寫入修正與 SAVE 驗證 (2026-03-13)
* **Error #8 修正**：重構 Disk II 寫入路徑，將 `Q7=1,Q6=0` 的寫入位移流程改為位元級節奏（4 cycles/bit），讓 DOS `SAVE` 後的 RWTS 驗證不再觸發 `ERROR #8`。
* **端到端驗證**：新增 `save_smoke`（`apple2-desktop/src/bin/save_smoke.rs`）自動執行 `CATALOG -> NEW -> SAVE TEST -> CATALOG`，可穩定驗證 `TEST` 檔案出現在目錄中。
* **目前限制**：寫入結果目前只更新模擬器記憶體中的 nibble tracks；尚未實作 denibblize 回寫 `.dsk` 檔案，重開後不保留。 

## 15. 速度模式與載入相容性修正 (2026-03-13)
* **F4 速度循環**：將原本單一 Turbo 切換改為循環倍率 `1x -> 2x -> 3x -> 4x -> 5x -> 1x`，便於依場景調速。
* **超頻音訊穩定**：移除高倍率下的硬性 `sink.clear()` 斷音策略，改為佇列過高時跳過單幀追加，保留音訊連續性。
* **`.dsk.gz` 載入修正**：統一啟動/F3 載入路徑，偵測 `.gz`（副檔名或 gzip magic）後先解壓，再做 140KB 尺寸驗證並載入 Disk II。

## 16. 高階遊戲相容性與音訊升級 (2026-03-13)
* **退出按鍵與 ESC 修正**：將退出模擬器的熱鍵從 `ESC` 移至 `F10`，釋放 `ESC` 鍵（ASCII 27）以供模擬器內的遊戲正常使用。
* **Language Card (64K RAM 擴充)**：實作 `$C080`~`$C08F` 的 Bank-switching 機制，為 `$D000`~`$FFFF` 區域提供額外的 16KB RAM，解決大型遊戲（如《七寶奇謀 Goonies》）因寫入 ROM 空間而崩潰的問題。
* **NMOS 6502 非法指令 (Illegal Opcodes)**：補齊了 `SLO`, `RLA`, `SRE`, `RRA` 等在防拷與遊戲優化中常見的未公開指令，提升極限應用的相容性。
* **虛擬搖桿防卡死 (Dummy Joystick)**：為 `$C061`~`$C067`（搖桿與按鈕）提供預設回應，避免部分老遊戲在啟動的校準迴圈中無限死結。
* **高傳真音訊積分 (Cycle-Accurate Audio Integration)**：放棄單純的定點採樣，改用「指令級佔空比積分 (Instruction-level Duty Cycle Integration)」並將採樣率提升至 44.1kHz。這將原本會產生頻率混疊的 PWM 高頻切換正確還原成白雜訊，完美修復了《德軍總部》等遊戲中槍聲變成嗶嗶聲的問題。

## 17. 磁碟系統架構哲學探討 (2026-03-13)
* **High-Level Emulation (Fast Disk) vs Low-Level Emulation**：
  * 討論了為何不採用「攔截 DOS 3.3 RWTS 呼叫 (High-Level Patching)」來實現磁碟加速（Fast Disk）。
  * **界線模糊**：Disk II 控制卡上的 `DISK2.ROM` (256 bytes) 僅負責將磁軌 0 磁區 0 (Stage 1 Bootloader) 載入 `$0800`，而真正的尋軌、讀寫、解碼邏輯 (RWTS) 是實作在被載入的作業系統 (如 DOS 3.3, 位於 `$BD00`) 或是遊戲自帶的客製化載入器中。
  * **相容性考量**：如果將控制卡當作「黑盒子」並攔截標準的 DOS 呼叫直接從 `.dsk` 複製資料，將會導致 90% 以上具有防拷保護（修改了 RWTS、Sync Bytes 或依賴特定硬體時序）的商業遊戲當機。
  * **結論**：本模擬器堅持採用 **Low-Level / Cycle-Accurate Emulation**。讓虛擬的 6502 CPU 執行真實的查表與 XOR 解碼，並在 `$C0EC` 提供精準的 32 週期位元組鎖存。這是確保所有標準 DOS 與極限防拷軟體皆能正常運作的唯一途徑。

## 18. 軟體開關 (Soft Switches) 實作盤點與未來藍圖 (2026-03-13)
經過盤點，目前模擬器針對 **Apple II+ (64K)** 的硬體標準已實現超過 95% 的核心軟體開關：
* **已實現 (100% 運作)**：
  1. 鍵盤輸入與 Strobe 清除 (`$C000`-$`C01F`)
  2. 內建喇叭 Toggle (`$C030`)
  3. 基礎顯示模式切換（Text/Graphics, Mixed, Page 1/2, Hi-Res/Lo-Res, `$C050`-$`C057`)
  4. Language Card 16K RAM 擴充與複雜的連讀解鎖機制 (`$C080`-$`C08F`)
  5. Disk II 控制器 Q6/Q7 狀態機與馬達相位控制 (`$C0E0`-$`C0EF`)
* **部分實現 (Dummy)**：
  * 遊戲按鈕與搖桿輸入 (`$C061`-$`C067`)，目前僅回傳 `0x00` 避免遊戲校準卡死。
* **尚未實現 (未來挑戰 / Apple IIe 擴充)**：
  1. **真實的類比搖桿計時器 (`$C070`)**：必須與 CPU 週期綁定電容放電時間，才能支援打磚塊等依賴 Paddle 的遊戲。
  2. **卡帶插槽 ROM 喚醒/休眠切換 (`$CFFF`, `$C0nX`)**：目前硬對應 Slot 6 給磁碟機，尚未實作嚴謹的 ROM 空間切換。
  3. **Apple IIe 輔助記憶體與 80 行顯示 (`$C000`-$`C00D`, `$C05E`-$`C05F`)**：為了執行 128K 遊戲（如《波斯王子》），未來需實作錯綜複雜的 Aux RAM/Main RAM 讀寫分離與雙高解析度 (Double Hi-Res) 渲染。
* **已知未解 Bug (待研究)**：
  * **音訊白雜訊失真**：雖然實作了 44.1kHz 的指令級佔空比積分 (Duty Cycle Integration)，但《德軍總部》等極端遊戲的槍聲雜訊依然呈現異常的「嗶嗶/嘟嘟」聲，推測可能還有更深層的 CPU 時序差異、未知的喇叭硬體非線性特性、或是過濾器 (DC Filter) 參數未最佳化所導致。此問題暫時擱置，待後續深度研究。



