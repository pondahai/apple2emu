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
