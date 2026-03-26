# Build Helpers

This directory contains helper scripts for building, installing, packaging, and cleaning the RTSP Viewer app.

## Windows Helpers

### Download

```
git clone https://github.com/firebadnofire/rtsp-webview.git
```

### Build

From the repository root, run:

```bat
build-helpers\build-windows-exe.bat
```

This script:

1. checks that `node`, `npm`, `cargo`, and `rustup` are installed
2. runs `npm ci` in `ui`
3. builds the frontend bundle
4. compiles the release Tauri app
5. copies the final executable to:

```text
dist\windows\rtsp-viewer.exe
```

Required Windows programs can be installed with:

```powershell
# Visual Studio 2022 Build Tools and Windows SDK
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart"

# Rustup
winget install Rustlang.Rustup
rustup default stable
rustup target add x86_64-pc-windows-msvc

# NodeJS
winget install OpenJS.NodeJS.LTS
```

Required Windows programs:

- Node.js LTS with npm
- rustup with a stable MSVC Rust toolchain
- Visual Studio 2022 Build Tools with Desktop development with C++
- Windows SDK installed through Visual Studio Build Tools

Recommended runtime dependency:

- Microsoft Edge WebView2 Runtime

### Install

After the build script has already produced `dist\windows\rtsp-viewer.exe`, run:

```bat
build-helpers\install-windows.bat
```

Behavior:

- requires that `dist\windows\rtsp-viewer.exe` already exists
- requests administrator elevation
- installs the app to `C:\Program Files\rtsp-viewer\rtsp-viewer.exe`
- prompts for Start Menu and Desktop shortcuts
- Enter defaults to `Y` for both prompts

### Uninstall

To remove the machine-wide installation, run:

```bat
build-helpers\uninstall-windows.bat
```

Behavior:

- requests administrator elevation
- removes `C:\Program Files\rtsp-viewer`
- removes the RTSP Viewer Start Menu and Desktop shortcuts if present

### Clean

To remove local Windows build output, run:

```bat
build-helpers\clean.bat
```

Removes generated local build output and helper state.

## Linux Tarball Helper

From the repository root, run:

```bash
./build-helpers/build-linux-tarball.sh
```

This repository includes a Docker-based Linux build pipeline for the Tauri desktop app.

By default it exports a `linux/amd64` tarball to `dist/linux/`.

To change the output directory:

```bash
./build-helpers/build-linux-tarball.sh /absolute/path/to/output
```

To build a different architecture:

```bash
BUILD_PLATFORM=linux/arm64 ./build-helpers/build-linux-tarball.sh
```

The exported tarball is named like:

```text
rtsp-viewer-0.1.0-linux-x86_64.tar.gz
```

The tarball includes the compiled binary, the project license, and Linux runtime notes.

## Unix-Style Clean

From the repository root, run:

```bash
make clean
```

This removes the same local build artifacts as the Windows cleaner and also tears down the Docker Buildx builder used by the Linux tarball pipeline when Docker is available.
