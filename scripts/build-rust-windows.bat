@echo off
REM Build the Rust core library for Windows
setlocal enabledelayedexpansion

set SCRIPT_DIR=%~dp0
set PROJECT_DIR=%SCRIPT_DIR%..
set RUST_DIR=%PROJECT_DIR%\rust-core

echo ==> Building rhythm-core for Windows...

cd /d "%RUST_DIR%"
cargo build --release --target x86_64-pc-windows-msvc

REM Copy outputs
set OUT_DIR=%PROJECT_DIR%\build\windows
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

copy "target\x86_64-pc-windows-msvc\release\rhythm_core.dll" "%OUT_DIR%\"
copy "target\x86_64-pc-windows-msvc\release\rhythm_core.dll.lib" "%OUT_DIR%\"
copy "include\rhythm_core.h" "%OUT_DIR%\"

echo ==> Output: %OUT_DIR%
echo     rhythm_core.dll
echo     rhythm_core.dll.lib
echo     rhythm_core.h

echo ==> Done!
