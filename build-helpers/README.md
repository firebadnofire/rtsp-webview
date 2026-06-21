# Build Helpers

This directory contains helper scripts for building, installing, packaging, and cleaning the RTSP Viewer app.

Current layout:

```text
build-helpers/
├── linux/
│   └── flatpak/
├── mac/
└── windows/
```

## Windows Helpers

### Download

```
git clone https://github.com/firebadnofire/rtsp-webview.git
```

### Build

From the repository root, run from cmd.exe (or double click):

```bat
build-helpers\windows\build-exe.bat
```

The script presents a numbered architecture menu before building:

1. `AMD64`
2. `x86`
3. `AARCH64`

To skip the prompt, pass the architecture directly via cmd:

```bat
build-helpers\windows\build-exe.bat aarch64
```

This script:

1. checks that Node.js 24 LTS, `npm`, `cargo`, and `rustup` are installed
2. verifies the selected Rust target is installed
3. runs `npm ci` in `ui`
4. builds the frontend bundle
5. compiles the release Tauri app for the selected architecture
6. copies the final executable to:

```text
dist\windows\rtsp-viewer.exe
```

It also keeps an architecture-specific copy under:

```text
dist\windows\<architecture>\rtsp-viewer.exe
```

The CI packaging workflow ships a portable Windows ZIP:

```text
dist/releases/rtsp-viewer-windows-amd64-portable.zip
```

It also builds a Windows installer executable on Linux using Tauri's experimental NSIS cross-compilation path:

```text
dist/releases/rtsp-viewer-windows-amd64-installer.exe
```

Required Windows programs can be installed with:

```powershell
# Visual Studio 2022 Build Tools and Windows SDK
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart"

# Rustup
winget install Rustlang.Rustup
rustup default stable
rustup target add x86_64-pc-windows-msvc
rustup target add i686-pc-windows-msvc
rustup target add aarch64-pc-windows-msvc

# Node.js 24 LTS
winget install OpenJS.NodeJS.LTS
```

Required Windows programs:

- Node.js 24 LTS with npm
- rustup with a stable MSVC Rust toolchain
- Visual Studio 2022 Build Tools with Desktop development with C++
- Windows SDK installed through Visual Studio Build Tools

Recommended runtime dependency:

- Microsoft Edge WebView2 Runtime

Installer note:

- the shipped Windows installer is an `.exe` produced by Tauri's NSIS bundler
- this Linux-hosted installer path is experimental in Tauri v1
- cross-platform signing is not supported in this path, so the installer will be unsigned unless the build strategy changes

### Install

After the build script has already produced `dist\windows\rtsp-viewer.exe`, run:

```bat
build-helpers\windows\install.bat
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
build-helpers\windows\uninstall.bat
```

Behavior:

- requests administrator elevation
- removes `C:\Program Files\rtsp-viewer`
- removes the RTSP Viewer Start Menu and Desktop shortcuts if present

### Clean

To remove local Windows build output, run:

```bat
build-helpers\windows\clean.bat
```

Removes generated local build output and helper state.

## Linux Tarball Helper

From the repository root, run:

```bash
./build-helpers/linux/build-tarball.sh
```

This repository includes a Docker-based Linux build pipeline for the Tauri desktop app. The builder image uses Node.js 24 LTS.

In non-interactive runs, it falls back to the current machine's native Linux architecture when recognized, otherwise `linux/amd64`.

Before building, the helper creates or reuses the external Docker network `build-system`, probes `http://apt-cacher-ng:3142`, and only passes an APT proxy into the Docker build when that cache is reachable. The temporary `buildx` builder is removed automatically when the script exits, so it does not sit idle afterward.

When run interactively, the helper now shows a numbered architecture menu before the Docker build starts:

1. `linux/amd64` (`x86_64`)
2. `linux/arm64` (`aarch64`)
3. `linux/arm/v7` (`armv7`)
4. `linux/ppc64le`
5. `linux/s390x`

To change the output directory:

```bash
./build-helpers/linux/build-tarball.sh /absolute/path/to/output
```

To skip the prompt and build a specific architecture directly:

```bash
BUILD_PLATFORM=linux/arm64 ./build-helpers/linux/build-tarball.sh
```

The exported tarball is named like:

```text
rtsp-viewer-0.1.0-linux-x86_64.tar.gz
```

The tarball includes the compiled binary, the project license, and Linux runtime notes.

The CI packaging workflow uses the generated `x86_64` tarball as an internal packaging input, then publishes this stable AppImage release artifact:

```text
dist/releases/rtsp-viewer-linux-x86_64.AppImage
```

## Linux Flatpak Helper

From the repository root, run:

```bash
./build-helpers/linux/flatpak/build-flatpak.sh
```

This helper uses the host `flatpak` CLI directly, so it does not require `flatpak-builder`.
It reuses the existing Docker-based Linux tarball build to avoid host-specific GTK/WebKit development package drift.

It will:

1. ensure `org.gnome.Platform//50` and `org.gnome.Sdk//50` are installed for the current user
2. build a Linux tarball in Docker for the current Flatpak architecture
3. extract the compiled release binary from that tarball
4. assemble a Flatpak app filesystem
5. export a local Flatpak repository
6. emit a single-file Flatpak bundle

Output:

```text
dist/flatpak/RTSP-Viewer-0.1.0-<arch>.flatpak
```

Runtime notes:

- the Flatpak keeps network access for RTSP streams
- the Flatpak currently exposes X11 display access and intentionally does not request the Wayland socket because the GTK/WebKit stack crashes on launch with Wayland enabled in this package
- the Flatpak is allowed to talk to `org.freedesktop.secrets` for stored credentials
- the helper bundles a dedicated `ffmpeg` executable under `/app/libexec/ffmpeg/bin/ffmpeg`
- startup restore uses the app-owned config directory instead of scanning the host filesystem

## macOS App Helper

On macOS, from the repository root, run:

```bash
./build-helpers/mac/build-app.sh
```

This script exits immediately on non-macOS systems.

It builds a universal macOS app for both Intel and Apple Silicon. It does not
present a target-selection prompt.

It uses the current machine's native macOS tooling to:

- run `npm ci`
- build the frontend bundle
- install the Rust `x86_64-apple-darwin` and `aarch64-apple-darwin` targets
- build both release Rust binaries
- combine them with `lipo`
- verify the final app executable contains both `x86_64` and `arm64`
- assemble `RTSP Viewer.app`
- try to generate an `.icns` file from `src-tauri/icons/icon.png`
- ad-hoc sign the app bundle with `codesign`

Output:

```text
dist/macos/RTSP Viewer.app
```

The CI packaging workflow zips that app bundle into:

```text
dist/releases/rtsp-viewer-macos-app.zip
```

If macOS quarantines the downloaded app after extraction, clear it before first launch:

```bash
xattr -dr com.apple.quarantine "/path/to/RTSP Viewer.app"
```

Runtime note:

- streaming still requires `ffmpeg`
- the macOS app now searches `PATH`, the app bundle directories, `/opt/homebrew/bin`, `/usr/local/bin`, and `/opt/local/bin`
- if `ffmpeg` is installed elsewhere, launch with `RTSP_VIEWER_FFMPEG=/absolute/path/to/ffmpeg`

Required tools:

- Node.js 24 LTS as `node`
- `npm`
- `cargo`
- `rustup`
- `lipo`
- `codesign`

The UI package enforces Node.js 24 during dependency installation.

Optional tools for a custom Finder icon:

- `sips`
- `iconutil`

## Linux Package Helper

After `dist/linux/rtsp-viewer-*.tar.gz` already exists, run:

```bash
./build-helpers/linux/build-package.sh
```

The script refuses to run if `dist/linux` does not contain a tarball.

It first presents an interactive numbered architecture menu to choose which tarball to package, then presents the package-format menu for that architecture.

Docker is still used for `.deb`, `.rpm`, and Arch packaging. Those formats create or reuse the external Docker network `build-system`, probe `http://apt-cacher-ng:3142`, and only enable an APT proxy when that cache is reachable. Their temporary `buildx` builder is removed automatically when the script exits.

AppImage packaging runs directly on the current build environment instead of Docker. It currently requires:

- `curl`
- `mksquashfs` from `squashfs-tools`

The package menu can build one of:

- `.deb`
- `.rpm` packages
- Arch `.pkg.tar.zst`
- `.AppImage`

Artifacts are exported to:

```text
dist/linux/packages/
```

The packaging containers default to the host's native Linux platform. The package architecture still comes from the tarball name itself, so an `x86_64` tarball still produces `x86_64` packages when the helper runs on Apple Silicon.

Tarball architecture support by package format currently looks like this:

- `x86_64` and `aarch64`: `.deb`, RPM, Arch package, and AppImage
- `armv7`: `.deb`, RPM, and Arch package
- `ppc64le` and `s390x`: `.deb` and RPM

The single `rpm` option builds both RPM variants into `dist/linux/packages/rpm/`:

- an `el9` RPM for RHEL-style systems
- an `opensuse` RPM for zypper/openSUSE-style systems

You can also skip one or both prompts and pass text tokens directly, with architecture first and package format second:

```bash
./build-helpers/linux/build-package.sh aarch64 deb
./build-helpers/linux/build-package.sh x86_64 rpm
./build-helpers/linux/build-package.sh armv7 arch
./build-helpers/linux/build-package.sh ppc64le deb
```

## Alpine / musl Status

This repository does not currently ship an Alpine/musl build helper.

The current desktop stack is Tauri 2 / WRY 0.55 / `webkit2gtk` 2.x, which targets the system WebKitGTK 4.1 / libsoup3 stack on Linux.

## Unix-Style Clean

### macOS

From the repository root, run:

```bash
./build-helpers/mac/clean.sh
```

### Linux

From the repository root, run:

```bash
./build-helpers/linux/clean.sh
```

Both Unix helpers remove the same local build artifacts as the Windows cleaner. The macOS helper delegates to the shared Unix cleanup routine, and the Linux helper also removes the Linux Docker builder state and the `build-system` network when Docker is available.
