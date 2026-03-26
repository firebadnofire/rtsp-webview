# rtsp-webview

## Local run

```bash
cd ui && npm ci && npm run build
cargo run
```
Build the frontend bundle on the host OS and run the Tauri app locally.

## Windows `.exe` build

Run the batch helper from the repository root in `cmd.exe` or PowerShell:

```bat
build-helpers\build-windows-exe.bat
```

The script will:

1. verify that `node`, `npm`, `cargo`, and `rustup` are installed
2. run `npm ci` in [`ui/package.json`](C:\Users\william\Desktop\git\rtsp-webview\ui\package.json)
3. build the frontend bundle
4. compile the Rust Tauri app in release mode
5. copy the final executable to `dist\windows\rtsp-viewer.exe`

### Required Windows programs

- `Node.js` LTS, which includes `npm`
- `rustup` with the stable MSVC Rust toolchain, for example `stable-x86_64-pc-windows-msvc`
- `Visual Studio 2022 Build Tools` with `Desktop development with C++`
- A Windows SDK installed through Visual Studio Build Tools

### Recommended runtime dependency

- `Microsoft Edge WebView2 Runtime` so the built Tauri app can launch on Windows

## Windows clean

Run the batch helper from the repository root:

```bat
build-helpers\clean.bat
```

It removes the same generated build artifacts as `make clean`, including `target`, `dist`, frontend build output, frontend dependencies, coverage output, vendored scratch space, helper state, and the Docker Buildx builder used by the Linux tarball helper when Docker is installed.

## Linux tarball

```bash
./build-helpers/build-linux-tarball.sh
```
Build the Dockerized Linux tarball at `dist/linux/rtsp-viewer-<version>-linux-<arch>.tar.gz`.

To change the output directory:

```bash
./build-helpers/build-linux-tarball.sh /absolute/path/to/output
```

## Cleaning

```bash
make clean
```

Remove local build artifacts, generated tarball output directories, Docker build cache for the Linux tarball pipeline, and the ignored `/vendor/` scratch directory.
