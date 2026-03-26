@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..\..") do set "REPO_ROOT=%%~fI"
set "STATE_DIR=%REPO_ROOT%\.build-helpers-state"
set "OUTPUT_REGISTRY=%STATE_DIR%\linux-tarball-output-dirs"
set "BUILDER_NAME=%BUILDER_NAME%"
if not defined BUILDER_NAME set "BUILDER_NAME=rtsp-webview-linux-builder"

set "REMOVED_ANY="

if exist "%OUTPUT_REGISTRY%" (
  for /f "usebackq delims=" %%P in ("%OUTPUT_REGISTRY%") do (
    if not "%%~P"=="" call :remove_path "%%~P"
  )
)

call :remove_path "%REPO_ROOT%\target"
call :remove_path "%REPO_ROOT%\dist"
call :remove_path "%REPO_ROOT%\ui\dist"
call :remove_path "%REPO_ROOT%\ui\node_modules"
call :remove_path "%REPO_ROOT%\ui\.vite"
call :remove_path "%REPO_ROOT%\coverage"
call :remove_path "%REPO_ROOT%\ui\coverage"
call :remove_path "%REPO_ROOT%\vendor"
call :remove_path "%STATE_DIR%"

where docker >nul 2>&1
if not errorlevel 1 (
  docker buildx inspect "%BUILDER_NAME%" >nul 2>&1
  if not errorlevel 1 (
    docker buildx prune --builder "%BUILDER_NAME%" --all --force >nul 2>&1
    docker buildx rm --force "%BUILDER_NAME%" >nul 2>&1
  )
)

if defined REMOVED_ANY (
  echo Build artifacts removed.
) else (
  echo Nothing to clean.
)
exit /b 0

:remove_path
set "TARGET=%~1"
if not defined TARGET exit /b 0
if /i "%TARGET%"=="\" exit /b 0
if /i "%TARGET%"=="%USERPROFILE%" exit /b 0
if not exist "%TARGET%" exit /b 0

attrib -R "%TARGET%" /S /D >nul 2>&1
rmdir /S /Q "%TARGET%" >nul 2>&1
if exist "%TARGET%" (
  del /F /Q "%TARGET%" >nul 2>&1
)
if exist "%TARGET%" exit /b 0

set "REMOVED_ANY=1"
exit /b 0
