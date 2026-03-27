# RTSP Viewer

# Why?

RTSP Viewer was born when I decided Lorex is bad at making software for desktops and browsers. 

My [D841 series](https://www.lorex.com/blogs/products/d441-series) DVR exposes an RTSP stream on it's LAN IP, so realizing the Lorex desktop app is both terrible and Windows only, I looked for alternatives on RTSP. Unfortunately, I never found a good one for multiple cameras. So that's what I made.

Originally, this was made as a [proof of concept in Python](https://codeberg.org/firebadnofire/rtsp-client.git). Eventually I grew tired with both python's limitations and the UI of the program itself. So I started over with this project in Rust with Webview.

# Features

`rtsp-webview` is a Rust desktop RTSP viewer with a WebView-based UI.

<details>
<summary>Program UI</summary>

![UI demo 1](img/uidemo1.png)
![UI demo 2](img/uidemo2.png)
![UI demo 3](img/uidemo3.png)

</details>

The current app supports:

- empty-first startup with no predefined screens
- manual camera setup
- bulk RTSP autoconfiguration from a template
- per-screen and global camera controls
- stored credentials
- snapshots and recording
- config save/load
- restoring active cameras when a saved config is loaded

## What The App Does

On first launch, the app starts with no screens and no panels.

From the empty workspace, users can:

- create the first screen manually
- open the bulk autoconfiguration tool

Each screen contains 4 panels in a 2x2 layout.

## Bulk Autoconfiguration

The bulk autoconfiguration tool generates camera panels from a user-provided RTSP URL template.

Supported placeholders:

- `$USERNAME`
- `$PASSWORD`
- `$IP`
- `$PORT`
- `$cameraNum`
- `$subNum`

Example template:

```text
rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum
```

The app asks for:

- username
- password
- IP
- port
- camera channel range
- subtype range

Generation behavior:

- one panel is created for each channel/subtype combination
- panels are assigned in order
- every 4 panels create a new screen automatically

Credential encoding behavior:

- `$USERNAME` and `$PASSWORD` are URL-encoded before substitution
- the entire RTSP URL is not encoded as one string

Example:

```text
Raw password:      testpw3@000
Encoded password:  testpw3%40000
```

## Config Autoload

On startup, the app looks for `rtsp_viewer_config.json` and automatically loads the first one it finds.

Current search order:

1. current working directory
2. executable directory
3. user home directory
4. Documents
5. Downloads
6. Pictures
7. Desktop
8. Music
9. Videos
10. Movies
11. repository root in debug builds

When a config is saved, any camera that is active at save time is marked to start again when that config is loaded later.

## Windows: Full Setup From `git clone` To Running The App

These steps assume:

- Windows 10 or Windows 11
- `winget` is available
- Git is already installed

### 1. Clone the repository

Open `cmd.exe`, PowerShell, or Windows Terminal and run:

```powershell
git clone https://github.com/<your-org-or-user>/rtsp-webview.git
cd rtsp-webview
```

If you already have the repository, just `cd` into it.

### 2. Install the required Windows build dependencies

Run these commands exactly:

```powershell
# Visual Studio 2022 Build Tools and Windows SDK
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart"

# Rustup
winget install Rustlang.Rustup
rustup default stable
rustup target add x86_64-pc-windows-msvc
rustup target add i686-pc-windows-msvc
rustup target add aarch64-pc-windows-msvc

# NodeJS
winget install OpenJS.NodeJS.LTS
```

After installation, close your terminal and open a new one so `node`, `npm`, `cargo`, and `rustup` are available on `PATH`.

### 3. Build the Windows executable

From the repository root, run:

```bat
build-helpers\windows\build-exe.bat
```

The helper presents a numbered architecture menu before building:

1. `AMD64`
2. `x86`
3. `AARCH64`

To skip the prompt, you can also run:

```bat
build-helpers\windows\build-exe.bat aarch64
```

What this script does:

1. checks that `node`, `npm`, `cargo`, and `rustup` are installed
2. verifies the selected Rust target is installed
3. runs `npm ci` in `ui`
4. builds the frontend bundle
5. builds the Rust/Tauri app in release mode for the selected architecture
6. copies the final executable to `dist\windows\rtsp-viewer.exe`

Expected build output:

```text
dist\windows\rtsp-viewer.exe
```

### 4. Run the built app directly

After the build finishes, run:

```powershell
.\dist\windows\rtsp-viewer.exe
```

### 5. Optionally install it into `Program Files`

If you want a machine-wide installation, run:

```bat
build-helpers\windows\install.bat
```

What it does:

1. errors out if `dist\windows\rtsp-viewer.exe` does not already exist
2. requests administrator elevation
3. installs the app to `C:\Program Files\rtsp-viewer\rtsp-viewer.exe`
4. asks whether to create Start Menu and Desktop shortcuts

Shortcut prompt behavior:

- type `Y` for yes
- type `N` for no
- press Enter to accept the default `Y`

### 6. Launch the installed app

After installation, you can launch the app by:

- running `C:\Program Files\rtsp-viewer\rtsp-viewer.exe`
- using the Start Menu shortcut if you created it
- using the Desktop shortcut if you created it

### 7. Uninstall it later if needed

Run:

```bat
build-helpers\windows\uninstall.bat
```

That script requests administrator elevation, removes `C:\Program Files\rtsp-viewer`, and deletes the RTSP Viewer Start Menu and Desktop shortcuts if they exist.

### Recommended Windows runtime dependency

Install the Microsoft Edge WebView2 Runtime if it is not already present on the machine. Tauri apps use WebView2 on Windows.

## Local Development Run

If you want to run the app directly from source instead of building the packaged Windows executable:

### Windows, Linux, or macOS

From the repository root:

```bash
cd ui
npm ci
npm run build
cd ..
cargo run
```

What this does:

1. installs frontend dependencies
2. builds the frontend bundle into `ui/dist`
3. starts the Tauri desktop app with `cargo run`

The app includes frontend preflight checks so development builds fail clearly if the frontend bundle is missing, empty, or unreachable instead of opening a blank white window.

## Development RTSP Test Server

A local RTSP test source is available for development:

```text
rtsp://test:testpw3%40000@127.0.0.1:5554/cam/realmonitor/cam/realmonitor?channel=1&subtype=0
```

Notes:

- raw password: `testpw3@000`
- encoded password in URLs: `testpw3%40000`
- supported channels: `1-16`
- supported subtypes: `0-1`

This makes it useful for validating:

- RTSP connectivity
- authentication handling
- URL encoding
- bulk autoconfiguration
- reconnect behavior
- snapshot behavior

To generate a full bulk configuration for the local test server, use:

```text
Template: rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor/cam/realmonitor?channel=$cameraNum&subtype=$subNum
Username: test
Password: testpw3@000
IP: 127.0.0.1
Port: 5554
Channels: 1-16
Subtypes: 0-1
```

That produces 32 panels across 8 screens.

## Linux Tarball Build

From the repository root:

```bash
./build-helpers/linux/build-tarball.sh
```

This builds the Docker-based Linux tarball and writes it under `dist/linux/`.

When run interactively, it shows a numbered architecture menu before the Docker build starts:

1. `linux/amd64` (`x86_64`)
2. `linux/arm64` (`aarch64`)
3. `linux/arm/v7` (`armv7`)
4. `linux/ppc64le`
5. `linux/s390x`

To choose a different output directory:

```bash
./build-helpers/linux/build-tarball.sh /absolute/path/to/output
```

To skip the prompt and set the architecture directly:

```bash
BUILD_PLATFORM=linux/arm64 ./build-helpers/linux/build-tarball.sh
```

## Cleaning Build Artifacts

### Windows

```bat
build-helpers\windows\clean.bat
```

### macOS

```bash
./build-helpers/mac/clean.sh
```

### Linux

```bash
./build-helpers/linux/clean.sh
```

These helpers remove local build artifacts such as:

- `target`
- `dist`
- `ui/dist`
- `ui/node_modules`
- coverage output
- helper state and scratch output

## Helper Script Reference

For a focused guide to the helper scripts, see [build-helpers/README.md](build-helpers/README.md).
