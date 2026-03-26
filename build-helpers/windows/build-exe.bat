@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..\..") do set "REPO_ROOT=%%~fI"

set "UI_DIR=%REPO_ROOT%\ui"
set "OUTPUT_DIR=%REPO_ROOT%\dist\windows"
set "RELEASE_EXE=%REPO_ROOT%\target\release\rtsp_viewer_tauri.exe"
set "OUTPUT_EXE=%OUTPUT_DIR%\rtsp-viewer.exe"

call :require_command node "Node.js"
if errorlevel 1 exit /b 1

call :require_command npm "npm"
if errorlevel 1 exit /b 1

call :require_command cargo "Rust"
if errorlevel 1 exit /b 1

call :require_command rustup "rustup"
if errorlevel 1 exit /b 1

echo [1/4] Checking Rust toolchain...
rustup default >nul 2>&1
if errorlevel 1 (
  echo ERROR: No default Rust toolchain is configured.
  echo Install one with: rustup toolchain install stable-x86_64-pc-windows-msvc
  echo Then set it with: rustup default stable-x86_64-pc-windows-msvc
  exit /b 1
)

echo [2/4] Installing frontend dependencies...
pushd "%UI_DIR%" >nul
call npm ci
if errorlevel 1 (
  popd >nul
  echo ERROR: npm ci failed.
  exit /b 1
)

echo [3/4] Building frontend bundle...
call npm run build
if errorlevel 1 (
  popd >nul
  echo ERROR: Frontend build failed.
  exit /b 1
)
popd >nul

echo [4/4] Building Windows executable...
pushd "%REPO_ROOT%" >nul
cargo build --release -p rtsp_viewer_tauri
if errorlevel 1 (
  popd >nul
  echo ERROR: Rust release build failed.
  echo If the error mentions MSVC, Windows SDK, or link.exe, install Visual Studio Build Tools with Desktop development with C++.
  exit /b 1
)
popd >nul

if not exist "%RELEASE_EXE%" (
  echo ERROR: Build finished but the expected executable was not found:
  echo   %RELEASE_EXE%
  exit /b 1
)

if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"
copy /Y "%RELEASE_EXE%" "%OUTPUT_EXE%" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy the executable to:
  echo   %OUTPUT_EXE%
  exit /b 1
)

echo Build complete.
echo Output executable:
echo   %OUTPUT_EXE%
exit /b 0

:require_command
where %~1 >nul 2>&1
if errorlevel 1 (
  echo ERROR: %~2 was not found in PATH.
  echo Install %~2 and open a new terminal before running this script again.
  exit /b 1
)
exit /b 0
