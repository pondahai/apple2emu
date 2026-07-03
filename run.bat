@echo off
REM One-click launcher: build (release) and open the Apple II emulator window.
REM Boots the last-used disk image (remembered in config.json).
REM In-app keys: F3 = load another disk, F2 = reboot, F4 = speed, F6/F7 = volume down/up, F10 = quit.
cd /d "%~dp0"

cargo run --release --bin apple2-desktop
if errorlevel 1 (
    echo.
    echo Build or launch failed - see the messages above.
    pause
)
