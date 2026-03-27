@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..\..") do set "REPO_ROOT=%%~fI"

set "UI_DIR=%REPO_ROOT%\ui"
set "OUTPUT_DIR=%REPO_ROOT%\dist\windows"
set "OUTPUT_EXE=%OUTPUT_DIR%\rtsp-viewer.exe"
set "ARCH_SELECTION=%~1"

call :select_architecture "%ARCH_SELECTION%"
if errorlevel 2 exit /b 0
if errorlevel 1 exit /b 1

set "OUTPUT_ARCH_DIR=%OUTPUT_DIR%\%ARCH_SLUG%"
set "OUTPUT_ARCH_EXE=%OUTPUT_ARCH_DIR%\rtsp-viewer.exe"
set "RELEASE_EXE=%REPO_ROOT%\target\%RUST_TARGET%\release\rtsp_viewer_tauri.exe"

call :require_command node "Node.js"
if errorlevel 1 exit /b 1

call :require_command npm "npm"
if errorlevel 1 exit /b 1

call :require_command cargo "Rust"
if errorlevel 1 exit /b 1

call :require_command rustup "rustup"
if errorlevel 1 exit /b 1

echo Building Windows executable for %ARCH_LABEL% ^(%RUST_TARGET%^).

echo [1/5] Checking Rust toolchain...
rustup default >nul 2>&1
if errorlevel 1 (
  echo ERROR: No default Rust toolchain is configured.
  echo Install one with: rustup toolchain install stable
  echo Then set it with: rustup default stable
  exit /b 1
)

echo [2/5] Checking Rust target...
rustup target add %RUST_TARGET% >nul 2>&1

echo [3/5] Installing frontend dependencies...
pushd "%UI_DIR%" >nul
call npm ci
if errorlevel 1 (
  popd >nul
  echo ERROR: npm ci failed.
  exit /b 1
)

echo [4/5] Building frontend bundle...
call npm run build
if errorlevel 1 (
  popd >nul
  echo ERROR: Frontend build failed.
  exit /b 1
)
popd >nul

echo [5/5] Building Windows executable...
pushd "%REPO_ROOT%" >nul
cargo build --locked --release --target %RUST_TARGET% -p rtsp_viewer_tauri
if errorlevel 1 (
  popd >nul
  echo ERROR: Rust release build failed.
  echo If the error mentions MSVC, Windows SDK, or link.exe, install Visual Studio Build Tools with Desktop development with C++ and the libraries for %ARCH_LABEL%.
  exit /b 1
)
popd >nul

if not exist "%RELEASE_EXE%" (
  echo ERROR: Build finished but the expected executable was not found:
  echo   %RELEASE_EXE%
  exit /b 1
)

if not exist "%OUTPUT_ARCH_DIR%" mkdir "%OUTPUT_ARCH_DIR%"
copy /Y "%RELEASE_EXE%" "%OUTPUT_ARCH_EXE%" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy the executable to:
  echo   %OUTPUT_ARCH_EXE%
  exit /b 1
)

copy /Y "%RELEASE_EXE%" "%OUTPUT_EXE%" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy the executable to:
  echo   %OUTPUT_EXE%
  exit /b 1
)

echo Build complete.
echo Output executable:
echo   %OUTPUT_EXE%
echo Architecture-specific copy:
echo   %OUTPUT_ARCH_EXE%
exit /b 0

:select_architecture
set "selection=%~1"
if not defined selection (
  echo Select a Windows build architecture:
  echo.
  echo 1. AMD64 ^(x86_64-pc-windows-msvc^)
  echo 2. x86 ^(i686-pc-windows-msvc^)
  echo 3. AARCH64 ^(aarch64-pc-windows-msvc^)
  echo 4. Quit
  echo.
  set /P "selection=Select a build architecture: "
)

if /I "%selection%"=="1" call :apply_architecture amd64 & exit /b !errorlevel!
if /I "%selection%"=="amd64" call :apply_architecture amd64 & exit /b !errorlevel!
if /I "%selection%"=="x64" call :apply_architecture amd64 & exit /b !errorlevel!
if /I "%selection%"=="x86_64" call :apply_architecture amd64 & exit /b !errorlevel!

if /I "%selection%"=="2" call :apply_architecture x86 & exit /b !errorlevel!
if /I "%selection%"=="x86" call :apply_architecture x86 & exit /b !errorlevel!
if /I "%selection%"=="i686" call :apply_architecture x86 & exit /b !errorlevel!
if /I "%selection%"=="386" call :apply_architecture x86 & exit /b !errorlevel!

if /I "%selection%"=="3" call :apply_architecture aarch64 & exit /b !errorlevel!
if /I "%selection%"=="aarch64" call :apply_architecture aarch64 & exit /b !errorlevel!
if /I "%selection%"=="arm64" call :apply_architecture aarch64 & exit /b !errorlevel!

if /I "%selection%"=="4" exit /b 2
if /I "%selection%"=="q" exit /b 2
if /I "%selection%"=="quit" exit /b 2
if /I "%selection%"=="exit" exit /b 2

echo ERROR: Invalid architecture selection "%selection%".
exit /b 1

:apply_architecture
if /I "%~1"=="amd64" (
  set "ARCH_SLUG=amd64"
  set "ARCH_LABEL=AMD64"
  set "RUST_TARGET=x86_64-pc-windows-msvc"
  exit /b 0
)

if /I "%~1"=="x86" (
  set "ARCH_SLUG=x86"
  set "ARCH_LABEL=x86"
  set "RUST_TARGET=i686-pc-windows-msvc"
  exit /b 0
)

if /I "%~1"=="aarch64" (
  set "ARCH_SLUG=aarch64"
  set "ARCH_LABEL=AARCH64"
  set "RUST_TARGET=aarch64-pc-windows-msvc"
  exit /b 0
)

echo ERROR: Unsupported Windows architecture "%~1".
exit /b 1

:require_command
where %~1 >nul 2>&1
if errorlevel 1 (
  echo ERROR: %~2 was not found in PATH.
  echo Install %~2 and open a new terminal before running this script again.
  exit /b 1
)
exit /b 0
