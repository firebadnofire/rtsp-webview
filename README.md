# rtsp-webview

## Local run

```bash
cd ui && npm ci && npm run build
cargo run
```
Build the frontend bundle on the host OS and run the Tauri app locally.

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
