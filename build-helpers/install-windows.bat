@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "REPO_ROOT=%%~fI"

set "SOURCE_EXE=%REPO_ROOT%\dist\windows\rtsp-viewer.exe"
set "INSTALL_DIR=%ProgramFiles%\rtsp-viewer"
set "INSTALL_EXE=%INSTALL_DIR%\rtsp-viewer.exe"
set "START_MENU_SHORTCUT=%ProgramData%\Microsoft\Windows\Start Menu\Programs\RTSP Viewer.lnk"
set "DESKTOP_SHORTCUT=%Public%\Desktop\RTSP Viewer.lnk"

if /i "%~1"=="--elevated" goto elevated

if not exist "%SOURCE_EXE%" (
  echo ERROR: Built executable not found:
  echo   %SOURCE_EXE%
  echo Run build-helpers\build-windows-exe.bat first.
  exit /b 1
)

call :prompt_yes_no CREATE_START_MENU "Create a Start Menu shortcut" "Y"
if errorlevel 1 exit /b 1

call :prompt_yes_no CREATE_DESKTOP "Create a Desktop shortcut" "Y"
if errorlevel 1 exit /b 1

echo Requesting administrator permissions...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "Start-Process -FilePath '%~f0' -Verb RunAs -ArgumentList '--elevated','%CREATE_START_MENU%','%CREATE_DESKTOP%'"
if errorlevel 1 (
  echo ERROR: Administrator elevation was cancelled or failed.
  exit /b 1
)
exit /b 0

:elevated
set "CREATE_START_MENU=%~2"
set "CREATE_DESKTOP=%~3"

if not defined CREATE_START_MENU set "CREATE_START_MENU=Y"
if not defined CREATE_DESKTOP set "CREATE_DESKTOP=Y"

net session >nul 2>&1
if errorlevel 1 (
  echo ERROR: Administrator permissions are required to install into %ProgramFiles%.
  exit /b 1
)

if not exist "%SOURCE_EXE%" (
  echo ERROR: Built executable not found:
  echo   %SOURCE_EXE%
  echo Run build-helpers\build-windows-exe.bat first.
  exit /b 1
)

if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
if errorlevel 1 (
  echo ERROR: Failed to create install directory:
  echo   %INSTALL_DIR%
  exit /b 1
)

copy /Y "%SOURCE_EXE%" "%INSTALL_EXE%" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy the executable to:
  echo   %INSTALL_EXE%
  exit /b 1
)

if /i "%CREATE_START_MENU%"=="Y" (
  call :create_shortcut "%START_MENU_SHORTCUT%"
  if errorlevel 1 exit /b 1
) else (
  if exist "%START_MENU_SHORTCUT%" del /F /Q "%START_MENU_SHORTCUT%" >nul 2>&1
)

if /i "%CREATE_DESKTOP%"=="Y" (
  call :create_shortcut "%DESKTOP_SHORTCUT%"
  if errorlevel 1 exit /b 1
) else (
  if exist "%DESKTOP_SHORTCUT%" del /F /Q "%DESKTOP_SHORTCUT%" >nul 2>&1
)

echo Install complete.
echo Installed executable:
echo   %INSTALL_EXE%
if /i "%CREATE_START_MENU%"=="Y" echo Start Menu shortcut: %START_MENU_SHORTCUT%
if /i "%CREATE_DESKTOP%"=="Y" echo Desktop shortcut: %DESKTOP_SHORTCUT%
exit /b 0

:prompt_yes_no
set "%~1="
set "USER_INPUT="
set /P USER_INPUT=%~2 [Y/n]: 
if not defined USER_INPUT set "USER_INPUT=%~3"
if /i "%USER_INPUT%"=="Y" (
  set "%~1=Y"
  exit /b 0
)
if /i "%USER_INPUT%"=="N" (
  set "%~1=N"
  exit /b 0
)
echo ERROR: Please answer Y or N.
exit /b 1

:create_shortcut
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$shortcutPath = [System.IO.Path]::GetFullPath('%~1');" ^
  "$shell = New-Object -ComObject WScript.Shell;" ^
  "$shortcut = $shell.CreateShortcut($shortcutPath);" ^
  "$shortcut.TargetPath = [System.IO.Path]::GetFullPath('%INSTALL_EXE%');" ^
  "$shortcut.WorkingDirectory = [System.IO.Path]::GetFullPath('%INSTALL_DIR%');" ^
  "$shortcut.IconLocation = [System.IO.Path]::GetFullPath('%INSTALL_EXE%');" ^
  "$shortcut.Save()"
if errorlevel 1 (
  echo ERROR: Failed to create shortcut:
  echo   %~1
  exit /b 1
)
exit /b 0
