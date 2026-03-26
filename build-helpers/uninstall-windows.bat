@echo off
setlocal EnableExtensions

set "INSTALL_DIR=%ProgramFiles%\rtsp-viewer"
set "INSTALL_EXE=%INSTALL_DIR%\rtsp-viewer.exe"
set "START_MENU_SHORTCUT=%ProgramData%\Microsoft\Windows\Start Menu\Programs\RTSP Viewer.lnk"
set "DESKTOP_SHORTCUT=%Public%\Desktop\RTSP Viewer.lnk"

if /i "%~1"=="--elevated" goto elevated

echo Requesting administrator permissions...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "Start-Process -FilePath '%~f0' -Verb RunAs -ArgumentList '--elevated'"
if errorlevel 1 (
  echo ERROR: Administrator elevation was cancelled or failed.
  exit /b 1
)
exit /b 0

:elevated
net session >nul 2>&1
if errorlevel 1 (
  echo ERROR: Administrator permissions are required to uninstall from %ProgramFiles%.
  exit /b 1
)

if exist "%START_MENU_SHORTCUT%" del /F /Q "%START_MENU_SHORTCUT%" >nul 2>&1
if exist "%DESKTOP_SHORTCUT%" del /F /Q "%DESKTOP_SHORTCUT%" >nul 2>&1

if exist "%INSTALL_EXE%" del /F /Q "%INSTALL_EXE%" >nul 2>&1

if exist "%INSTALL_DIR%" (
  rmdir /S /Q "%INSTALL_DIR%" >nul 2>&1
)

if exist "%INSTALL_DIR%" (
  echo ERROR: Failed to remove install directory:
  echo   %INSTALL_DIR%
  exit /b 1
)

echo Uninstall complete.
echo Removed install directory:
echo   %INSTALL_DIR%
exit /b 0
