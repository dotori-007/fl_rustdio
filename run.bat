@echo off
chcp 65001 > nul
title FL Rustdio - PureRust Audio Studio

echo ========================================================
echo   FL Rustdio - Pure Rust Native Audio Studio
echo ========================================================
echo.

cd /d "%~dp0"

if exist "target\release\fl_rustdio.exe" (
    echo [INFO] 릴리즈 빌드 실행 파일을 실행합니다...
    start "" "target\release\fl_rustdio.exe"
) else if exist "target\release\rust_mp3_editor.exe" (
    echo [INFO] 릴리즈 빌드 실행 파일을 실행합니다...
    start "" "target\release\rust_mp3_editor.exe"
) else (
    echo [INFO] 빌드된 실행 파일이 없습니다. Cargo로 실행합니다...
    cargo run --release
)

exit
