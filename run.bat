@echo off
REM One-click launcher: build (release) and open the Apple II emulator window.
REM Boots the last-used disk image (remembered in config.json).
REM In-app keys: F1 = warm reset (Ctrl-Reset), F2 = reboot (cold), F3 = load another disk, F5 = speed, F7 = color/green screen, F8/F9 = volume down/up, F10 = quit.
cd /d "%~dp0"

cargo run --release --bin apple2-desktop
if errorlevel 1 (
    echo.
    echo Build or launch failed - see the messages above.
    pause
)
