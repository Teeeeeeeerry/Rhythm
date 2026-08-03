@echo off
REM Build the full Windows application
setlocal enabledelayedexpansion

set SCRIPT_DIR=%~dp0
set PROJECT_DIR=%SCRIPT_DIR%..
set RUST_DIR=%PROJECT_DIR%\rust-core
set WIN_DIR=%PROJECT_DIR%\windows

echo === Step 1: Build Rust core DLL ===
cd /d "%RUST_DIR%"
cargo build --release --target x86_64-pc-windows-msvc
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo === Step 2: Copy DLL and import lib ===
set DLL_SRC=%PROJECT_DIR%\target\x86_64-pc-windows-msvc\release\rhythm_core.dll
set LIB_SRC=%PROJECT_DIR%\target\x86_64-pc-windows-msvc\release\rhythm_core.dll.lib
set HEADER_SRC=%RUST_DIR%\include\rhythm_core.h

copy "%DLL_SRC%" "%WIN_DIR%\Rhythm\Bridge\" /Y
copy "%LIB_SRC%" "%WIN_DIR%\Rhythm\Bridge\" /Y
copy "%HEADER_SRC%" "%WIN_DIR%\Rhythm\Bridge\" /Y

echo === Step 3: Build Windows app with CMake ===
cd /d "%WIN_DIR%"
if not exist "build" mkdir build
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release

echo === Done ===
echo App: %WIN_DIR%\build\Release\Rhythm.exe
