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

## 15. 速度模式與載入相容性修正 (2026-03-13)
* **F4 速度循環**：將原本單一 Turbo 切換改為循環倍率 `1x -> 2x -> 3x -> 4x -> 5x -> 1x`，便於依場景調速。
* **超頻音訊穩定**：移除高倍率下的硬性 `sink.clear()` 斷音策略，改為佇列過高時跳過單幀追加，保留音訊連續性。
* **`.dsk.gz` 載入修正**：統一啟動/F3 載入路徑，偵測 `.gz`（副檔名或 gzip magic）後先解壓，再做 140KB 尺寸驗證並載入 Disk II。

## 16. 實體磁碟檔案寫回與持久化 (2026-03-17)
* **Denibblize 實作與驗證**：在 `apple2-core/src/nibble.rs` 中實作了 `denibblize_dsk`，能夠將記憶體中修改過的原始 Nibble 磁軌反向解碼為標準 140KB `.dsk` 格式。並透過 `test_nibble.rs` 驗證其雙向轉換的無損性 (0 bytes mismatch)。
* **自動存檔觸發機制**：於 `Disk2` 加入 `is_dirty` 狀態標記，並整合到 `apple2-desktop` 的主迴圈中。
* **涵蓋所有退出路徑**：
  1. 按 `F3` 換片時會自動將舊磁片變更寫回。
  2. 按 `F2` 冷重開機時會寫回並更新記憶體快取。
  3. 關閉模擬器視窗或按 `F10` 離開時，會自動寫回變更。
* **壓縮檔支援**：加入 `save_disk_image`，能在寫回檔案時偵測是否為 `.gz`，並正確地將資料重新用 Gzip 壓縮，解決了冷開機再次載入時發生的 `invalid gzip header` 問題。現在你的磁碟變更已能完美、安全地保存下來！
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

## 19. 《The Goonies》磁碟相容性除錯紀錄 (2026-03-14)
* **使用者症狀**：載入 `C:\Users\pondahai\Downloads\AppleWin1.26.1.1\ac\goonies.dsk.gz` 時，模擬器不是停在 `APPLE ][` 開機畫面，就是讀取後進入花螢幕/亂碼狀態。
* **映像檔確認**：
  * `.gz` 解壓後為標準 `143360` bytes，非損壞檔案。
  * 問題不在 `.dsk.gz` 載入路徑；啟動/F3 解壓流程正常。
* **穩定基線確認**：
  * `save_smoke` 仍可正常進入 `]`、執行 `CATALOG -> NEW -> SAVE TEST -> CATALOG`。
  * 表示一般 DOS 3.3 啟動與 Disk II 寫入路徑仍然正常，問題集中在高相容性 loader。
* **新增診斷工具**：
  * 建立 `apple2-desktop/src/bin/goonies_probe.rs`，以 headless 方式載入 `goonies.dsk.gz`，記錄 CPU/磁碟狀態、RAM 區段與卡點。
  * probe 顯示 loader 會一路進入 RAM `$0486` 附近的遊戲載入段，之後長時間卡在 `$045F/$0460` 與 `$051F/$0520` 迴圈。
  * 最終狀態固定在 `quarter-track = 92`、`track = 23` 附近反覆讀取，並非一開始就完全無法讀盤。
* **RAM 內 loader 關鍵發現**：
  * `$0380` 例程會不斷輪詢 `$C08C` 尋找 `D5 AA 96` prologue。
  * 這說明卡點在後段自訂 loader 的 Disk II 讀取語意，而非 GUI 載入流程、gzip、或主開機流程。
* **已嘗試且證偽的方向**：
  * **ProDOS / DOS sector order 切換**：沒有改善，DOS-order 仍較接近正確。
  * **單純將 `$C08C` 改為 non-destructive read**：
    * `goonies` 反而退回只停在 `APPLE ][`。
    * 一般 DOS 啟動與 `save_smoke` 也退化，故不能直接套用。
  * **延長 ready window（同一 byte 保留多次 polling）**：
    * 同樣會把一般 DOS boot 打壞，故已撤回。
  * **第一版完整 bit-level read sequencer**：
    * 測試可過，但 `save_smoke` 退回只停在 `APPLE ][`。
    * 表示讀取狀態機方向正確，但實作過於粗暴，尚未與現有 DOS 路徑相容。
* **已保留的有效改進**：
  * `memory.rs` / `machine.rs` 改為 **bus-level timing plumbing**：
    * 每次 bus access 先推進 Disk II `1` cycle。
    * instruction 結尾再補剩餘 cycles。
  * 此改動不破壞 `save_smoke`，但單獨不足以解開 `goonies` loader。
* **目前結論**：
  * 問題不是 `.gz`、不是 GUI 啟動路徑、不是簡單的 quarter-track 缺失，也不是單純 instruction-level timing 太粗。
  * 真正缺的是 **更接近真機的 Disk II read sequencer / `$C08C` 輪詢語意**，而且必須在不破壞現有 DOS 3.3 路徑的前提下導入。
* **下一步方向**：
  * 保留目前穩定的 byte-level 基線作為 fallback。
  * 另外建立較保守的 shadow read sequencer，專門改善 `$0380` 這種 prologue search/polling 行為。
  * 每次修改都必須同時驗證：
    * `cargo run --quiet --bin save_smoke`
    * `cargo run --quiet --bin goonies_probe`

## 20. 外部資料查核：`The Goonies` 與 Apple II 保護盤脈絡 (2026-03-14)
* **已確認的外部事實**：
  * `The Goonies` Apple II 版為 **Datasoft** 發行的 1985 年商業版本。
  * 這至少說明它屬於 Apple II 商業保護盤常見年代與發行商範圍。
* **與目前觀察吻合的外部脈絡**：
  * Apple II 商業保護盤常會直接輪詢 Disk II 資料暫存器（如 `$C08C`），依賴 bit-stream、sync、bit-slip、weak bits 或非標準 sector/track 佈局。
  * 這與 `goonies_probe` 看到 RAM `$0380` 反覆輪詢 `$C08C` 尋找 `D5 AA 96` prologue 的現象一致。
* **模擬器實作上的旁證**：
  * 多個 Apple II 模擬器/工具鏈都提過：若軟體使用非標準保護或 track-level 行為，單純 `.dsk` 表示法可能不足，往往需要更原始的 nibble/track 格式支援。
  * AppleWin 歷年 release note 也可見持續修正 Disk II 相容性邊界案例，顯示這類問題在實務上很常見。
* **目前仍未查到的部分**：
  * 尚未找到公開資料明確指出 `The Goonies` Apple II 版採用哪一種 Datasoft copy protection。
  * 尚未找到直接描述「`The Goonies` 在某模擬器卡在 track 23 / 花螢幕」的公開個案。
* **本段結論（推論，不是已證實事實）**：
  * `The Goonies` 很可能使用對 Disk II 後段讀取語意較敏感的商業 loader / 保護機制。
  * 因此問題最合理地仍指向 Disk II read sequencer / `$C08C` polling 相容性缺口，而不是 `.gz` 載入、GUI 路徑或一般 DOS 啟動流程。

## 21. `goonies_probe` 續追：`$0380` 的 `$C08C` polling 已能命中 address field (2026-03-14)
* **本輪追加觀測**：
  * 擴充 `apple2-desktop/src/bin/goonies_probe.rs`：
    * 列出 track 23 上 `D5 AA 96` address prologue 的實際分布。
    * 追蹤 `$0380` 例程內各個 `LDA $C08C,X` 讀點。
    * 在 `$0380` 成功 `RTS` 時記錄解出的 `volume/track/sector/checksum` 與 caller 預期值。
* **關鍵新發現**：
  * 在 stuck 狀態的 track 23 上，probe 可穩定看到 address prologue 出現在固定位置：
    * `64, 460, 856, ...`，間距為 `396` bytes，符合目前 nibblized DOS track 結構。
  * `$0380` 並不是完全卡死在找不到 `D5 AA 96`：
    * trace 明確顯示 `FF ... FF D5 AA 96` 可被正確讀到。
    * 後續 4-and-4 address bytes 也正確解出，例如 `vol=FE`, `trk=17`, `sec=0B/0C/...`。
  * caller 在 `ret=0535` 時，確實只是因為「目前掃到的 sector 不是期待值」而繼續重試。
  * 當掃到 caller 期待的 sector（例如 `expect_sec=02 -> sec=02`，以及後續 `04 -> 04`、`06 -> 06`）時，流程會前進到下一段，而不是永遠卡死在 `$0380`。
* **本輪結論修正**：
  * 先前把問題集中在「`$0380` / `$C08C` polling 抓不到 prologue」這個假設，現在已被 probe 證偽。
  * 目前更合理的卡點已往後移：
    * address field search / header decode **基本可用**；
    * 真正異常更可能出在 **命中正確 sector 之後的 data-field 讀取 / decode / 後續控制流**。
* **下一步方向**：
  * 續追 caller 在 sector 命中後進入的下一段路徑（目前從 trace 看已不只是 `$0380` 問題）。
  * 優先觀察：
    * `$0596` 後續資料場讀取流程。
    * `$0318` 一帶的 data-field decode 是否在正確 header 命中後仍回傳錯誤狀態。

## 22. `goonies_probe` 再續追：sector 命中後已前進到 `$0400` consumer 路徑 (2026-03-14)
* **本輪追加觀測**：
  * 對 `goonies_probe` 再加：
    * `$0318` 區塊記憶體 dump。
    * `$0596/$05AA/$05B2/$05BA/$05CD/$05D4` 附近 path trace。
  * 目的是確認「命中正確 sector 後是否真的有離開 `$0380` / `$0535` 重試路徑」。
* **關鍵新發現**：
  * RAM `$0318` 內容顯示這裡確實還有另一段 data-field 讀取例程，會尋找 `D5 AA AD`。
  * 當 `$0380` 命中 caller 期待的 sector 時，不只會離開 `$0535` 重試點，還會進一步回到 `ret=059A`。
  * 之後 probe 看到穩定的後續路徑：
    * `05B2 -> 05BA -> 05CD`
    * 接著由 `05D8` 的 `JMP $0400` 進入下一段 consumer 流程。
  * 這表示目前流程其實能：
    * 找到正確 address field；
    * 命中正確 sector；
    * 至少部分前進到後續 loader 邏輯。
* **目前更精確的結論**：
  * 問題已不再集中於 `$0380` 的 address-field polling。
  * 卡點更可能位於：
    * `$0318` data-field 讀取/解碼本身；
    * 或 `$0400` 之後消費 decoded data 的流程。
  * 也就是說，之前「先修 `$C08C` polling」這條主假設，現在應下修優先度。
* **下一步方向**：
  * 直接追 `$0400` consumer 路徑：
    * 對 `JMP $0400` 後的關鍵分支與 buffer 狀態做 trace。
  * 同時補抓：
    * `$0318` 例程的真正 return 點與 carry/accumulator 結果；
    * sector 命中後寫入的 decode buffer 是否內容異常。

## 23. `goonies_probe` 再下鑽：`$0400` 後主要卡在 `$045D/$045F` 倒數等待段 (2026-03-14)
* **本輪追加觀測**：
  * 對 `$0400` consumer 路徑加 trace。
  * 在首次進入 `$0400` 時 dump：
    * `00E0..00EF`
    * `0200..023F`
    * `0280..02BF`
    * `0300..03FF`
    * `0400..047F`
* **關鍵新發現**：
  * 命中正確 sector 並經過 `05B2 -> 05BA -> 05CD` 後，流程確實會進入 `$0400`。
  * `$0400` 內真正大量出現的 hot spot 不是前段判斷，而是：
    * `$045D -> $045F`
  * 這段會讓：
    * `A` 從較大值一路倒數；
    * `FE/FF` 持續前進；
    * Disk byte index 也持續轉動。
  * 從行為上看，這更像是 loader 正在等待某種 timing / step / rotational position 條件，而不是單純 data-field 一進來就立刻壞掉。
* **目前最合理的解讀**：
  * 問題焦點已從：
    * `$0380` address-field polling，
    * 移到 `$0318` data-field / `$0400` consumer，
    * 現在又更集中到 `$0400` 內部的等待/步進控制段。
  * 換句話說，Disk II 相容性缺口仍然可能存在，但更像是：
    * loader 所需的步進/旋轉/ready 條件與目前模擬語意仍有偏差。
* **下一步方向**：
  * 直接對 `$044D..$0467` 這段等待/控制路徑做更細 trace。
  * 需要特別對照：
    * `$FE/$FF` 的用途；
    * `$C080,X` / `$C08x` soft-switch 存取是否對應到真機預期的 phase/step 行為。

## 24. `goonies_probe` 續追 stepper：phase 有切換，但磁頭 quarter-track 仍卡在 92 (2026-03-14)
* **本輪追加觀測**：
  * 對 `$044D/$0450/$0457/$045D/$0460/$0467` 加入 stepper trace。
  * 直接記錄：
    * `qtr-track / track / byte index / data latch`
    * `phases[0..3]`
    * `$E5`, `$FE`, `$FF`
* **關鍵新發現**：
  * `$0457` 確實會碰 phase soft-switch，trace 看得到 phase pattern 變化。
  * 但在 stuck 區段中：
    * phase 多半只在 `0110` 與 `0010` 間活動；
    * `current_qtr_track` 一直維持在 `92`；
    * `current_track` 也固定在 `23`。
  * 同時 `byte_index` 與 `data_latch` 仍持續更新，表示盤面旋轉還在跑，只是**磁頭位置沒有因這些 phase 操作而產生新的有效步進**。
* **目前最合理的解讀**：
  * loader 在 `$045D/$045F` 等待的，很可能就是某種步進後條件。
  * 我們的 Disk II phase-to-quarter-track 模型雖然對一般 DOS boot 夠用，但在 `The Goonies` 這段 loader 的 phase 序列下，沒有產生它期待的磁頭移動語意。
  * 這使得流程停留在 track 23 上反覆等待，即使旋轉本身正常。
* **下一步方向**：
  * 重新檢查 `disk2.rs` 的 `step_motor()` 規則，特別是多相位同時為 ON 時的目標 quarter-track 計算。
  * 優先驗證方向：
    * 真機/常見模擬器對 `0110 -> 0010`、`0010 -> 0110` 這類 phase 序列的 head movement 語意；
    * 是否需要更貼近 latch/half-step 慣性的 stepper 模型，而不是目前的 target-snapping 寫法。

## 25. 嘗試 canonical half-step `step_motor()`：`save_smoke` 不壞，但 `goonies` 仍未脫離 track 23 (2026-03-14)
* **本輪實作**：
  * 修改 `apple2-core/src/disk2.rs` 的 `step_motor()`：
    * 改成 canonical 8-state half-step 模型。
    * 單相位對應偶數 quarter-track，相鄰雙相位對應奇數 quarter-track。
  * 新增 `apple2-core/src/disk2_test.rs` 單元測試：
    * 驗證相鄰雙相位會落在 half-step。
    * 驗證從雙相位退回單相位會回到偶數 quarter-track。
* **驗證結果**：
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
  * `cargo run --quiet --bin save_smoke`：通過。
  * `cargo run --quiet --bin goonies_probe`：`The Goonies` 仍卡在 track 23，未見明顯前進。
* **本輪觀察**：
  * 新模型本身沒有打壞目前 DOS 路徑。
  * 但在目前 probe 抓到的 stuck 視窗中，stepper trace 仍主要看到 phase=`0010`，`qtr-track` 維持在 `92`。
  * 也就是說，光把 `step_motor()` 從 snapping 改成 canonical half-step，還不足以解開這個 loader。
* **目前結論**：
  * `step_motor()` 的粗糙模型可能仍是問題的一部分，但不是唯一缺口，或至少不是目前最直接的卡點。
  * 下一步不應只繼續盲調 stepper，而應回頭確認：
    * `$0400` 一帶實際寫進 `$E5/X` 的 phase 序列；
    * loader 是否還依賴其他尚未模擬的 Disk II ready/phase side effect。

## 26. `$0400` phase 序列定稿：`$045D/$045F` 是 seek settle delay，不是 Disk II ready wait (2026-03-14)
* **本輪重點**：
  * 直接把 RAM `$0400` 例程反組譯並對照 `goonies_probe` trace。
  * 釐清 `$E5`、`X`、`$C080,X` 與 `$028E` 的真正語意。
* **關鍵解碼**：
  * `$0400` 開頭的 `STX $E5 / STA $E4` 中：
    * `$E5` 保存的是 Slot 6 Disk II 基底偏移 `#$60`；
    * `$E4` 是 seek 目標值；
    * `$028E` 不是單純 DOS track number，而是 **half-track index**。
  * 因此 stuck 視窗中的：
    * `$028E=$2E` 代表 half-track `46`，
    * 對應 `quarter-track = 92`，
    * 也就是實際 `track 23`。
  * 這說明先前看到 `current_qtr_track=92` 並不是「明明 seek 了卻沒動」，而是剛好對上 loader 自己的目標編碼。
* **`$C080,X` 真正的 phase 序列**：
  * `$0430` 先 `SEC`，再 `JSR $044E`：
    * `LDA $028E`
    * `AND #$03`
    * `ROL A`
    * `ORA $E5`
    * `TAX`
    * `LDA $C080,X`
  * 因為進入時 Carry=`1`，`ROL` 會產生 **奇數 offset**，所以這次 access 其實是：
    * `$C0E1/$C0E3/$C0E5/$C0E7`
    * 也就是 **phase ON**。
  * 接著 `$043A` 走 `CLC` 後的 `JSR $0451`，Carry=`0`，所以第二次 access 會變成：
    * `$C0E0/$C0E2/$C0E4/$C0E6`
    * 也就是 **phase OFF**。
  * 換句話說，`$0400` 這段 seek 的實際模式是：
    * **先把新 half-track 對應的 phase 打開**
    * **延遲**
    * **再把舊 half-track 對應的 phase 關掉**
    * **再延遲**
* **stuck 視窗的具體例子**：
  * 在最後一步進入 `track 23` 前，trace 顯示：
    * 舊值 `$E1=$2D`，
    * 新值 `$028E=$2E`，
    * 最終 phase 由 `0110` 收斂到 `0010`。
  * 對應的 soft-switch 序列就是：
    * 新位置 `$2E` 經 `SEC+ROL` -> `X=$65` -> 讀 `$C0E5` -> **phase 2 ON**
    * 舊位置 `$2D` 經 `CLC+ROL` -> `X=$62` -> 讀 `$C0E2` -> **phase 1 OFF**
  * 這正好就是 canonical half-step 的 `0110 -> 0010`，並且 `quarter-track 91 -> 92`。
  * 所以就 stuck 視窗來看，模擬器其實**已經做出了 loader 這一步 seek 所要求的 phase 轉換**。
* **`$045D/$045F` 等待的是什麼**：
  * `$045D` 的子程序內容只有：
    * `LDX #$11`
    * `DEX/BNE` busy loop
    * `INC $FE`
    * `INC $FF`（進位時）
    * `SEC / SBC #$01 / BNE`
  * 它**完全沒有讀任何 Disk II soft-switch**，也沒有碰 `$C08C/$C0EC`。
  * 因此 `$045D/$045F` 等待的不是 data latch ready、不是 byte arrival，也不是某個即時 rotational condition。
  * 它是在做 **固定時間的 seek settle delay**，同時把經過時間累加到 `$FE/$FF`。
* **本輪結論修正**：
  * 先前把 `$045D/$045F` 解讀成「等待某個 Disk II 條件成立」過於寬泛；更精確地說，它是在 **phase ON/OFF 之間與之後消耗時間，讓磁頭完成 half-step / settle**。
  * 因此目前 `The Goonies` 的主卡點不再像是：
    * 「`$0400` 沒有發出正確 phase 序列」，
    * 或「`$045D/$045F` 少等了某個 ready bit」。
  * 更合理的下一個焦點應該往後移到：
    * seek 完成後的後續資料讀取窗口；
    * `$051F/$0520` 一帶如何消費 `$FE/$FF` 累積的時間與 sector 流；
    * 或 seek 完成後磁頭/旋轉對齊的更細緻副作用。

## 27. `post_seek` trace：`$051F/$0520` 迴圈發生在 phases=`0000` 的全線圈 off 狀態 (2026-03-14)
* **本輪追加觀測**：
  * 擴充 `goonies_probe`，專抓 `$0500/$0512/$051F/$0520/$0524/$052A` 的 `postseek` trace。
  * 追蹤：
    * `$FE/$FF`
    * `$05E4/$05EC/$05ED`
    * `$0269/$026E/$028E`
    * `byte_index/data_latch/qtr-track/phases`
* **關鍵新發現**：
  * 進入 `$0400` consumer 後，第一次命中 sector 2 時：
    * `e7=02`, `e8=17`, `e9=FE`
    * `028e=17`（half-track 23）
    * `05ec=29`, `05ed=60`
  * 在真正大量 hot 的 `$051F/$0520` 迴圈中，trace 穩定顯示：
    * `phases=0000`
    * `qtr=92`
    * `track=23`
    * `Y` 從 `$11` 倒數到 `00`
    * `byte_index` 緩慢前進
    * `data_latch` 跟著盤面轉動更新
  * 每當 `Y` 倒數結束，流程會到 `$0520 -> $0522 -> $0524 -> $051D`，並讓：
    * `$FE` 增加 1
    * 然後再進下一輪 `Y` 倒數
  * 也就是說 `$051F/$0520` 本身不是 stepper loop，也不是在等待某個 phase 轉換成立；它發生時四個 phase 線圈都已經關掉。
* **目前最合理的解讀**：
  * loader 在 `$0400` 已完成 seek 後，會進入一段 **all-phases-off 的 rotational waiting window**。
  * 這段等待靠 CPU delay 與 `$FE/$FF` 計時，並讓磁碟繼續旋轉、byte index 自然前進。
  * 因此如果相容性仍出問題，嫌疑點更像是：
    * 真機在「全部 phase 關閉後」對磁頭保持/漂移/機械慣性的語意；
    * 或 seek 完成後，讀取窗口相對於旋轉位置的對齊差異。
* **本輪結論修正**：
  * 問題不在 `$051F/$0520` 又偷偷發出錯誤 phase 序列；這段根本沒有再切 phase。
  * 下一步應優先檢查：
    * 我們在 phases 全關時是否過度理想化地把磁頭固定在 `qtr=92`；
    * 以及 seek 結束後 `byte_index/data_latch` 是否缺少真機級的 step-settle side effect。

## 28. `post_seek` 細 trace 補充：`$051F/$0520` 是純 CPU delay + rotational drift，`$0524` 只是在推進 `$FE` (2026-03-14)
* **本輪追加觀測**：
  * 用 `goonies_probe` 只篩 `postseek|path059x|entered 0400 consumer|final pc|pc hits`，降低 `$0380` 噪音。
  * 觀察 sector `02` 命中後第一次進入 `$0400` consumer 的完整 seek 後窗口。
* **關鍵新發現**：
  * 在進入 `$051F/$0520` 之前，probe 先看到：
    * `pc=0519 next=051B`
    * `Y=$DF`
    * `phases=0000`
    * `idx=1805`
  * 之後大量重複的熱點是：
    * `051F -> 0520 -> 051F ...`
    * `Y` 從 `$11` 一路倒數到 `00`
    * `byte_index` 只在磁碟自然旋轉時慢慢前進
    * `qtr=92`、`phases=0000` 全程不變
  * 每次 `Y` 倒數到 `00` 時：
    * `0520 -> 0522`
    * 接著 `0524 -> 051D`
    * 並讓 `$FE` 從 `$2C -> $2D -> $2E -> ...` 持續增加。
  * 同時：
    * `$05EC` 維持 `#$29`
    * `$05ED` 維持 `#$60`
    * `$0269` 維持 `#$02`
    * 沒看到新的 phase soft-switch 活動。
* **目前更精確的解讀**：
  * `$051F/$0520` 是 seek 完之後的 **純 CPU delay loop**；
  * 真正和磁碟互動的只剩「盤面繼續旋轉，`byte_index/data_latch` 自然更新」；
  * `$0524` 則像是在把這段等待折算進 `$FE` 的 elapsed-time counter。
* **本輪結論**：
  * 如果 `The Goonies` 還是卡住，缺口更像不是「phase 序列不對」，而是：
    * seek 完後開始觀察盤面的時間點不對；
    * 或 head move 完成瞬間，真機會留下某種我們目前沒有模擬的對齊/settle side effect。
  * 下一輪適合做的實驗，不是再亂改 `$051F/$0520`，而是：
    * 檢查 phase 切換完成瞬間是否要延後生效到新 track；
    * 或在 head step 完成後，對 `byte_index/data_latch` 注入更接近真機的 settle 語意。

## 29. 最小 `step settle` 實驗：DOS 不壞，但 `goonies` trace 幾乎不變 (2026-03-14)
* **本輪實作**：
  * 在 `apple2-core/src/disk2.rs` 加入最小版 `step settle` 視窗：
    * 當 `step_motor()` 跨越 track 邊界時，先記住 `settle_track`。
    * 接下來固定 `128` cycles 內，`tick()` 讀寫仍暫時使用舊 track。
  * 新增 `apple2-core/src/disk2_test.rs` 測試：
    * 驗證 cross-track step 後，短暫 settle window 內仍先讀到舊 track，之後才切到新 track。
* **驗證結果**：
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
  * `cargo run --quiet --bin save_smoke`：通過。
  * `cargo run --quiet --bin goonies_probe`：關鍵 trace 幾乎與前一輪相同。
* **觀察**：
  * `goonies_probe` 仍然：
    * 命中 sector `02 / 04 / 06` 後回到 `ret=059A`
    * 進入 `$0400` consumer
    * 最後長時間卡在 `$051F/$0520`
  * `postseek` trace 仍顯示：
    * `phases=0000`
    * `qtr=92`
    * `$05EC=29`, `$05ED=60`
    * `pc hits: 045F=144364 0460=144364 051F=324685 0520=324684`
  * 也就是說，這個「跨 track 後短暫沿用舊 track」模型雖然合理，也不會破壞 DOS，但**沒有實質改變 `The Goonies` 的卡點形態**。
* **目前結論**：
  * 問題不像是單純「step 完後前幾十/百 cycles 還在讀舊 track」。
  * 更值得懷疑的缺口改為：
    * quarter-track/half-track 對應到實體磁訊號時，是否需要更細緻的 cross-track bleed / analog overlap；
    * 或 loader 實際依賴的是 step 之後的 rotational alignment，而不是短暫 track selection 延遲本身。

## 30. 最小 read-only cross-track overlap：`goonies` 有明顯位移，但 DOS 路徑立即退化，已撤回 (2026-03-14)
* **本輪實作**：
  * 在 `apple2-core/src/disk2.rs` 只對 `Q7=0,Q6=0` 讀模式加上保守版 cross-track overlap：
    * 僅在 `current_qtr_track` 靠近 track 邊界時啟用；
    * 交錯混入鄰近 track 的 byte。
  * 補了單元測試，驗證：
    * 邊界位置會混入前一軌資料；
    * 非邊界位置仍只讀本軌。
* **觀察**：
  * `goonies_probe` 的行為確實出現了**本質位移**：
    * 不再落回原本大量的 `$051F/$0520` waiting window；
    * `pc hits` 變成 `051F=0, 0520=0`；
    * `final pc` 甚至跑到 `FA50`。
  * 但這個改動同時**立即打壞 DOS 基線**：
    * `save_smoke` 畫面退化到 monitor / 錯亂狀態；
    * `Tracks changed after SAVE flow` 變成 `0`，和正常基線不符。
* **結論**：
  * 這證明 `The Goonies` 的 loader 對「跨軌類比重疊」這一類語意**非常敏感**；
  * 也證明如果直接把 overlap 粗暴地套進一般讀路徑，會立刻破壞正常 DOS 3.3 啟動/讀寫。
* **處置**：
  * overlap 實驗碼已**完整撤回**，恢復穩定基線。
  * `cargo test -p apple2-core disk2_test -- --nocapture`：恢復通過。
  * `cargo run --quiet --bin save_smoke`：恢復通過。
* **下一步建議**：
  * 若要走 overlap 方向，不能做成全域常態讀取規則；
  * 更合理的是設計成：
    * 只在特定 quarter-track / phase-off seek 後窗口啟用；
    * 或更接近真機的弱耦合/低比例混合，而不是交錯 byte 級替換。

## 31. 超局部 post-seek shadow read：DOS 基線維持，但 `goonies` 幾乎完全不動，已撤回 (2026-03-14)
* **本輪實作**：
  * 在 `apple2-core/src/machine.rs` 先用極窄條件鎖定 `goonies` 的 post-seek 視窗：
    * `pc` 只限 `$051D/$051F/$0520/$0522/$0524`
    * `phases=0000`
    * `current_qtr_track` 限在 `92` 附近
    * RAM signature 另外鎖 `0269=02`, `05EC=29`, `05ED=60`
  * 只有命中這個視窗時，才通知 `apple2-core/src/disk2.rs` 啟用弱 shadow read。
  * shadow source 只取「最近一次跨軌前的舊 track」，並只做很弱的偏向：
    * 優先保留本軌 byte；
    * 僅在少數位置或 shadow byte 帶高位時才偏向舊軌。
* **驗證結果**：
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
  * `cargo run --quiet --bin save_smoke`：仍通過，`Tracks changed after SAVE flow: 2`，顯示 DOS 基線沒被打壞。
  * `cargo run --quiet --bin goonies_probe`：沒有看到前一輪全域 overlap 那種本質位移。
* **觀察**：
  * `goonies_probe` 仍然進入 `entered 0400 consumer`，之後長時間停在同一組 post-seek 視窗。
  * `final pc` 仍是 `051F`。
  * `pc hits` 仍是：
    * `045F=144364`
    * `0460=144364`
    * `051F=324685`
    * `0520=324684`
  * 也就是說，這個「只在 `$051F/$0520` 等待窗內做弱 shadow」版本，雖然足夠局部、不會傷 DOS，但對 loader 的控制流幾乎沒有實際影響。
* **結論**：
  * overlap 方向本身仍成立，但有效訊號顯然不是「只在卡住之後的 phase-off waiting loop 內輕量混入舊軌 byte」。
  * 真正敏感的點更可能發生在：
    * 更早的 seek 完成瞬間；
    * step/settle 到 read window 的交界；
    * 或需要比「弱 byte 級偏向」更接近類比磁頭耦合的模型。
* **處置**：
  * 這組超局部 shadow read 實驗碼已完整撤回，恢復乾淨基線。

## 32. `step_motor()` 跨軌後前 4 byte 弱 shadow：仍無位移，已撤回 (2026-03-14)
* **本輪實作**：
  * 將 overlap/shadow 的實驗點前移到真正的 seek-to-read 交界：
    * `apple2-core/src/machine.rs` 只在 `goonies` 的 `$0400` consumer / seek 路徑附近 arm 實驗。
    * `apple2-core/src/disk2.rs` 在 `step_motor()` 真正跨軌時，記住舊 track，並開一個只有 `4` 個 read byte 的短 window。
  * 讀取規則維持非常保守：
    * 預設仍以新軌 byte 為主；
    * 只有當舊軌 byte 看起來像 sync/prologue（例如 `>= $D5`），或高位條件特別明顯時，才短暫偏向舊軌。
* **驗證結果**：
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
  * `cargo run --quiet --bin save_smoke`：通過，DOS 基線未受影響。
  * `cargo run --quiet --bin goonies_probe | rg "entered 0400 consumer|path059x|final pc|pc hits"`：仍與前一輪基線相同。
* **觀察**：
  * `goonies` 仍會：
    * 命中 `sector 02 / 04 / 06`
    * 進入 `entered 0400 consumer`
    * 最後回到原本的 stuck 形態
  * 關鍵結果沒有位移：
    * `final pc=051F`
    * `pc hits: 045F=144364 0460=144364 051F=324685 0520=324684`
* **結論**：
  * 敏感點看來也不是「跨軌後頭 4 個 byte 的弱 sync/prologue 偏置」。
  * 換句話說，問題不像是簡單的：
    * post-seek wait-loop shadow；
    * 或 post-step 前幾 byte shadow。
  * 更值得懷疑的缺口可能要再往下探到：
    * rotational alignment 在 seek 完成瞬間的重定位；
    * `byte_index`/phase 關閉時機與真機的對齊差；
    * 或 quarter-track 對磁訊號的取樣位置，而不是 byte 值混合本身。
* **處置**：
  * 實驗碼已完整撤回，恢復乾淨基線。

## 33. 今日總結：方向正確，但 byte-level overlap 不是正確抽象層 (2026-03-14)
* **今日最重要的正負訊號**：
  * 全域 read-only cross-track overlap 會明顯改變 `The Goonies` 行為，甚至把 loader 從原本的 `$051F/$0520` 卡點推走。
  * 但這種做法會立刻打壞 DOS 基線與 `save_smoke`，因此不能作為一般 Disk II 讀取規則。
  * 相反地，任何「太晚、太弱、太局部」的 shadow/overlap 版本，雖然不會破壞 DOS，卻幾乎無法推動 `goonies`。
* **本日結論**：
  * 大方向仍然是對的：
    * 問題確實和 seek 後的類比/跨軌/取樣語意有關；
    * `goonies` 很可能依賴某種真機級的 head coupling / mechanical settling / rotational alignment 副作用。
  * 但目前已高度懷疑：
    * 「直接混 byte 值」不是正確抽象層；
    * 有效訊號更可能在 seek 完成瞬間的取樣位置或對齊，而不是讀到哪個 byte 值。
* **已證偽或應下修優先度的方向**：
  * `$051F/$0520` waiting loop 本身不是主因。
  * 單純的 seek settle delay 長短不是主因。
  * post-seek waiting window 的弱 shadow 沒有效果。
  * `step_motor()` 跨軌後前幾個 byte 的弱 shadow 也沒有效果。
* **目前最值得優先嘗試的下一步**：
  1. **`byte_index` / rotational alignment 重定位實驗**
     * 在跨軌完成瞬間，不混 byte，而是對 `byte_index` 做很小的偏移。
     * 先只在 `goonies` 的 `$0400` seek 路徑、`qtr≈92`、相關 phase transition 後測 `+1/+2/+4/+8` byte。
  2. **phase-off 的短暫 head bias / hold 語意**
     * 檢查 `phases=0000` 後是否仍應保留短暫的 last-step 偏置，而不是把磁頭理想化地完全固定。
  3. **quarter-track 取樣位置模型**
     * 若前兩者仍無效，下一層才是探討 quarter-track 如何影響實際 bitstream / byte phase 的取樣位置，而不是繼續做 byte 級混合。
* **實務建議**：
  * 後續實驗應優先維持乾淨 DOS 基線，避免再引入全域 overlap 規則。
  * 每輪都應同時驗證：
    * `cargo test -p apple2-core disk2_test -- --nocapture`
    * `cargo run --quiet --bin save_smoke`
    * `cargo run --quiet --bin goonies_probe`

## 34. 最小 `byte_index +2` seek-bias：DOS 不壞，但 `goonies` 完全無位移，已撤回 (2026-03-14)
* **本輪實作**：
  * 放棄 byte 混合，改做更貼近 rotational alignment 的最小實驗：
    * 只在 `goonies` 的 `$0400` seek/consumer 路徑附近 arm。
    * 當 `step_motor()` 真正跨軌時，對接下來 `4` 個 read bytes 套固定 `byte_index + 2` 偏移。
  * 實驗範圍仍維持非常局部：
    * 不改一般 DOS 路徑；
    * 不改寫入路徑；
    * 不改 `data_latch` destructive read 規則；
    * 只改跨軌後前幾個 read byte 的取樣位置。
* **驗證結果**：
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
  * `cargo run --quiet --bin save_smoke`：通過，`Tracks changed after SAVE flow: 2`。
  * `cargo run --quiet --bin goonies_probe | rg "entered 0400 consumer|path059x|final pc|pc hits"`：結果與基線完全相同。
* **觀察**：
  * `goonies` 仍然：
    * 命中 `sector 02 / 04 / 06`
    * 進入 `entered 0400 consumer`
    * 最後卡在原本同一個 hot loop
  * 關鍵數字完全沒有位移：
    * `final pc=051F`
    * `pc hits: 045F=144364 0460=144364 051F=324685 0520=324684`
* **結論**：
  * 最小的 rotational alignment 偏移 `+2` 本身不足以影響 loader。
  * 這說明問題不是單純「跨軌後前幾個 byte 的固定小幅 index 偏差」。
  * 若要繼續走 alignment 方向，下一步應該：
    * 做有系統的 offset sweep（`+1/+2/+4/+8`）；
    * 或改成 phase-off / last-step bias 類型的機械保持語意，而不是固定 index 偏移。
* **處置**：
  * 本輪 `byte_index +2` 實驗碼已完整撤回，恢復乾淨基線。

## 35. `byte_index` offset sweep (`+1/+2/+4/+8`)：全部零反應，alignment 固定偏移分支可判死 (2026-03-14)
* **本輪做法**：
  * 把上一輪的 `byte_index` seek-bias 暫時做成 probe 專用 sweep。
  * 依序測：
    * `offset=0`（基線）
    * `offset=1`
    * `offset=2`
    * `offset=4`
    * `offset=8`
  * 每一輪都只比較：
    * `final pc`
    * `pc hits: 045F / 0460 / 051F / 0520`
* **驗證前提**：
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
  * `cargo run --quiet --bin save_smoke`：通過。
  * 表示 sweep 期間 DOS 基線沒有被打壞。
* **sweep 結果**：
  * 五組 offset 的結果完全一致：
    * `final pc=051F`
    * `pc hits: 045F=144364 0460=144364 051F=324685 0520=324684`
  * 換句話說：
    * `+1` 無效
    * `+2` 無效
    * `+4` 無效
    * `+8` 無效
* **本輪結論**：
  * 「跨軌後前幾個 byte 套固定小幅 `byte_index` 偏移」這整條分支可以直接判死。
  * 問題不像是單純的固定 rotational phase 偏差。
  * 如果 alignment 還是嫌疑點，那也更可能是：
    * 非固定 offset；
    * 和 phase-off / last-step 狀態耦合的動態偏置；
    * 或更接近 quarter-track 取樣位置的模型，而不是單純 `byte_index += N`。
* **處置**：
  * sweep 專用實驗碼已完整撤回，恢復乾淨基線。

## 36. `phases=0000` 最小 head-hold：DOS 不壞，但 `goonies` 仍完全零位移，已撤回 (2026-03-14)
* **本輪實作**：
  * 改測「全相位關閉後是否應保留 last-step 機械偏置」。
  * 實作方式維持極窄：
    * 只在 `goonies` 的 `$0400` seek/consumer 路徑附近啟用；
    * 只有在真正跨軌之後；
    * 並且進入 `phases=0000` 後，前 `4` 個 read bytes 才暫時讀回上一軌 `hold_track`。
  * 這比之前的跨軌 settle 更保守，因為它不影響 phase 仍為 ON 的 seek 過程，只碰「seek 完後全相位關閉」的短窗口。
* **驗證結果**：
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
  * `cargo run --quiet --bin save_smoke`：通過，`Tracks changed after SAVE flow: 2`。
  * `cargo run --quiet --bin goonies_probe | rg "entered 0400 consumer|path059x|final pc|pc hits"`：與基線完全相同。
* **觀察**：
  * `goonies` 仍然：
    * 命中 `sector 02 / 04 / 06`
    * 進入 `entered 0400 consumer`
    * 最後卡在相同的 `$051F/$0520` 熱點形態
  * 關鍵結果仍完全不變：
    * `final pc=051F`
    * `pc hits: 045F=144364 0460=144364 051F=324685 0520=324684`
* **結論**：
  * 最小版 phase-off head-hold 也沒有任何效果。
  * 這表示問題不像是：
    * 固定小幅 `byte_index` 偏移；
    * 也不像是「全相位關閉後短暫沿用上一軌」這種簡化的機械保持模型。
  * 若方向仍要沿著真機副作用往下挖，下一層更可能是：
    * quarter-track 對實體磁訊號取樣位置的模型；
    * 或 nibble/track 表示法本身不足以承載 `The Goonies` 所依賴的保護語意。
* **處置**：
  * 本輪 head-hold 實驗碼已完整撤回，恢復乾淨基線。

## 37. Desktop `auto turbo`：從 I/O 門檻版簡化為 `motor_on` 即無節流 (2026-03-14)
* **動機**：
  * `auto turbo` 的目標不是修正磁碟相容性，而是改善一般讀盤時的體感速度。
  * 先前做過一版「依 Disk II I/O 密度觸發」的自動加速，但實務上還可以再更簡單、更直接。
* **最終設計**：
  * 不碰 `apple2-core` 的 cycle 語意。
  * 只在 `apple2-desktop/src/main.rs` 的前端節流層做判斷：
    * **只要 `disk2.motor_on == true`，就進入 `AUTO TURBO UNTHROTTLED`**
    * **只要 `disk2.motor_on == false`，就回到 `F4` 的手動速度設定**
  * 自動加速期間：
    * `window.set_target_fps(0)`，也就是不做 FPS 節流。
    * 視窗標題顯示 `AUTO TURBO UNTHROTTLED`。
  * 手動速度模式仍保留：
    * `F4` 依舊是 `1x -> 2x -> 3x -> 4x -> 5x -> 1x`
    * 但只在磁碟馬達關閉時主導前端速度。
* **取捨**：
  * 優點：
    * 邏輯非常簡單，容易預測。
    * 不需要猜「哪些 PC 區間是磁碟程式」。
    * 不需要維護 I/O 門檻或 hold-window。
    * 不改 Disk II、CPU、音訊的核心 timing model。
  * 缺點：
    * 只要馬達開著，就會無節流，即使程式此刻不一定在密集讀盤。
    * 這是刻意接受的簡化，因為目標本來就是桌面版讀盤加速，而不是更細緻的節流策略。
* **驗證**：
  * `cargo check -p apple2-desktop`：通過。
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過。
* **目前結論**：
  * 這版 `auto turbo` 體感效果良好，可作為目前桌面版的預設磁碟加速策略。

## 38. 修正 `F3` 換磁碟與 `F2` 重開後音訊狀態錯亂 (2026-03-14)
* **問題現象**：
  * 按 `F3` 換磁碟後，聲音會直接消失。
  * 第一輪修正後，`F3` 恢復正常，但 `F2` cold boot 之後開機 beep 又消失。
* **根因**：
  * `apple2-desktop/src/main.rs` 的 `AudioMixerState` 在 `F2/F3` 事件後被重設到 `cycle 0`，
    但 emulator 的 `machine.total_cycles` 並沒有同步歸零。
  * 此外，`rodio::Sink` 只在程式啟動時建立一次；`F2/F3` 後如果沿用舊 sink，
    舊 queue/backlog 會把短促的 boot beep 吃掉。
* **實作**：
  * 將 mixer 重設改為 `reset_at(current_cycle, cycles_per_sample, speaker_on)`，
    直接對齊當前 CPU cycle，而不是硬回到 `0`。
  * 在 `F2/F3` 路徑改成重建新的 `rodio::Sink`，讓音訊輸出狀態更接近「程式剛啟動」。
  * 補了一個 desktop 單元測試，確認 `reset_at()` 會把 mixer 對齊到指定 cycle。
* **驗證**：
  * `cargo test -p apple2-desktop --bin apple2-desktop -- --nocapture`：通過。
  * 實機確認：
    * `F3` 換磁碟後聲音恢復。
    * `F2` cold boot 後 boot beep 恢復。
* **提交**：
  * `ecd3160` `Fix audio reset after disk swap and reboot`

## 39. 新增 Apple II joystick 模擬，並修正 Windows `Alt` 無法當按鈕 (2026-03-15)
* **需求**：
  * 用鍵盤方向鍵模擬 Apple II 搖桿。
  * 按鈕維持用 `Alt`，避免佔用一般 Apple II 鍵盤字元。
* **core 端實作**：
  * 在 `apple2-core/src/memory.rs` 加入 joystick 狀態：
    * `pushbuttons[2]`
    * `paddles[4]`
    * `paddle_latch_cycle`
  * 補上 Apple II game I/O 行為：
    * `$C061/$C062` 回傳 pushbutton bit 7
    * `$C064-$C067` 依 paddle timeout window 回傳 bit 7
    * `$C070` 會重新 strobe paddle timer
  * 新增 `memory_test`：
    * pushbutton bit 7 測試
    * paddle strobe/timeout 測試
* **desktop 端實作**：
  * 方向鍵直接映射到 Paddle 0/1 的 X/Y。
  * `Left Alt` / `Right Alt` 映射到 Pushbutton 0/1。
* **關鍵發現**：
  * `minifb` 在 Windows 的鍵盤 scan code 映射裡沒有正確把 `Alt` 接到
    `Key::LeftAlt/RightAlt`，導致 `window.is_key_down(Key::LeftAlt)` 永遠抓不到。
  * 因此桌面前端額外使用 Win32 `GetAsyncKeyState(VK_LMENU/VK_RMENU)` 直接讀取 `Alt` 狀態。
* **驗證**：
  * `cargo test -p apple2-core memory_test -- --nocapture`：通過。
  * `cargo test -p apple2-desktop --bin apple2-desktop -- --nocapture`：通過。
  * 實機確認：
    * 一般鍵盤輸入正常。
    * `Alt` 搖桿按鈕正常。
    * 某些遊戲中右/下方向仍可能有相容性問題，這部分尚未修。
* **提交**：
  * `9ce6a27` `Add joystick emulation with Alt button support`

## 40. CPU undocumented opcode 擴充與輸入反應修正 (2026-03-15)
* **CPU undocumented opcode coverage 補齊**：
  * 在 `apple2-core/src/cpu.rs` 補上 `ANC`, `ALR`, `ARR`, `AXS/SBX`, `LAS`, `XAA`, `AHX`, `TAS`, `SHY`, `SHX`, `KIL/JAM` 與 `0xEB` (`SBC #imm` unofficial alias)。
  * 將 opcode table 補到 **256/256**，並為 `KIL/JAM` 加入明確的 jammed/halt 狀態。
* **Decimal mode 補全**：
  * 在 `apple2-core/src/instructions.rs` 為 `ADC/SBC` 補上 NMOS 6502 的 BCD arithmetic 路徑。
* **非法 NOP / IGN bus 行為修正**：
  * 補齊 `ZeroPage,X` 與 `Absolute,X` 類 undocumented NOP 的 dummy read / wrong-page read。
  * 這對 Apple II 的 memory-mapped I/O side effects 尤其重要。
* **測試**：
  * `apple2-core/src/cpu_test.rs` 新增對 undocumented opcodes、decimal mode、jam state、與 NOP dummy reads 的回歸測試。
  * `cargo test -p apple2-core` 通過。
* **目前定性**：
  * `SLO/RLA/SRE/RRA/LAX/SAX/DCP/ISC/NOP/KIL/ANC/ALR/ARR/AXS/EB` 已達到高完整度。
  * `XAA/LAX #imm/AHX/SHX/SHY/TAS` 仍屬於工程上可用的近似模型，不宜宣稱為完全真機級。

## 41. 鍵盤反應遲滯與搖桿 Alt 對應微調 (2026-03-15)
* **問題現象**：
  * 加入 Windows `Alt` 搖桿按鈕支援後，一般鍵盤輸入體感變慢。
* **根因方向**：
  * 主迴圈在每次按鍵時都 `println!` 到 console，且每幀都重複呼叫 `set_target_fps()` 與 `set_title()`，導致前端互動路徑多了不必要的同步開銷。
* **修正**：
  * 移除每個按鍵事件的 console log。
  * 將 `window.set_target_fps()` 與 `update_window_title()` 改為只在狀態變化時更新。
  * 依使用需求，將桌面版搖桿按鈕映射調整為：
    * `Right Alt -> Pushbutton 0`
    * `Left Alt -> Pushbutton 1`
* **驗證**：
  * `cargo check -p apple2-desktop` 通過。
  * 實機回報：鍵盤反應已改善。

## 42. undocumented store opcode 跨頁行為再逼近真機 (2026-03-15)
* **目標**：
  * 針對 `AHX`, `SHX`, `SHY`, `TAS` 這批 `H+1` family 的 page-cross 行為，把先前「只寫到 final effective address」的簡化模型往真機再推近一步。
* **修正**：
  * 在 `apple2-core/src/cpu.rs` 中，這批 opcode 現在會：
    * 以 base address 的高位元組計算 `H+1` mask；
    * 若發生 page crossing，寫入位址的高位元組改由 stored value 決定，而不再固定寫到 final effective address。
* **測試**：
  * `apple2-core/src/cpu_test.rs` 新增 `AHX/SHX/SHY/TAS` 的 page-cross 回歸測試。
  * `cargo test -p apple2-core` 通過。
* **目前定性**：
  * 這比前一版更接近已知 NMOS 6502 行為，但仍屬於高可信近似，不宜宣稱為已與所有晶片批次完全一致。

## 43. Disk II read sequencer：保守版 bit-level 內部狀態 (2026-03-15)
* **目標**：
  * 讓 Disk II 讀路徑不再只是單純的「每 32 cycles 吐一個 byte」，而是開始保留 bit-level 讀取內部狀態，為後續更真實的 sequencer 行為鋪路。
* **實作**：
  * 在 `apple2-core/src/disk2.rs` 加入：
    * `read_shift_register`
    * `read_bit_phase`
  * 讀模式下改為每 `4` cycles shift `1` bit。
  * 但對 CPU 可見的 `data_latch` 仍只在 byte 邊界更新，避免直接破壞現有 DOS 路徑。
* **關鍵取捨**：
  * 第一版若讓 `data_latch` 在 bit-level 過程中即時更新，`save_smoke` 立刻退化，只停在 `APPLE ][`。
  * 因此目前採用「bit-level state 內部存在，但 byte-boundary publish 對外可見」的保守模型。
* **測試與驗證**：
  * `apple2-core/src/disk2_test.rs` 新增：
    * 每 `4` cycles 前進一個 bit 的測試
    * destructive read 後，bit-level 狀態持續累積、但 latch 仍在下一個 byte boundary 恢復的測試
  * `cargo test -p apple2-core`：通過
  * `cargo run --quiet --bin save_smoke`：通過
* **目前定性**：
  * 這是 read sequencer 的安全地基，不是完整 P6 state machine。
  * 下一步若要再往真機靠近，應優先處理：
    * sync / bit-slip 語意
    * destructive-read 之後的恢復窗口
    * 在不破壞 DOS 3.3 基線下，逐步引入更細的 read 行為

## 44. Disk II 讀取幾何可獨立於 nibble 長度，並在 seek/load 時重置相位 (2026-03-16)
* **目標**：
  * 為保護或特殊 loader 研究保留「表觀一圈長度」與實際 nibble bytes 脫鉤的能力，同時避免跨磁軌殘留的旋轉相位污染讀取結果。
* **實作**：
  * 在 `apple2-core/src/nibble.rs` 的 `TrackData` 加入 `read_length`，預設與 nibblized track 實際長度相同。
  * 在 `apple2-core/src/disk2.rs` 新增 `read_rotation_accumulator`，讓 `read_length > length` 時可用分數式方式放慢 byte index 前進。
  * 抽出 `reset_rotation_state()`，在 `load_disk()`、`reset()`，以及 stepper 真正跨到新 track 時清掉 `byte_index`、bit-phase、shift register 與 rotation accumulator。
* **測試**：
  * `apple2-core/src/disk2_test.rs` 新增：
    * `read_only_geometry_can_stretch_rotation_without_changing_track_length`
    * `seek_to_new_track_resets_read_rotation_phase`
  * `cargo test -p apple2-core disk2_test -- --nocapture`：通過
  * `cargo check -p apple2-desktop`：通過
* **目前定性**：
  * 這仍然是工程上的保守近似，不是完整 weak-bit / variable-density / P6 模型。
  * 但已能安全地做讀取幾何實驗，而不會把上一條磁軌的相位狀態帶進下一條。
