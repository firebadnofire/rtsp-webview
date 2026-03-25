# Build Helpers

## Linux tarball pipeline

This repository includes a Docker-based Linux build pipeline for the Tauri desktop app.

The Docker build applies the required Linux `wry` patch inside the container, so the repository does not need a tracked `vendor/` tree.

The tarball helper uses a dedicated Buildx builder named `rtsp-webview-linux-builder`, and `make clean` removes that builder, its cache, and any output directories recorded by prior tarball builds.

Run it from the repository root:

```bash
./build-helpers/build-linux-tarball.sh
```

By default the script builds a `linux/amd64` artifact and exports it to `dist/linux/`.

To change the output directory:

```bash
./build-helpers/build-linux-tarball.sh /absolute/path/to/output
```

To build a different Linux architecture with Docker Buildx:

```bash
BUILD_PLATFORM=linux/arm64 ./build-helpers/build-linux-tarball.sh
```

The exported artifact is a gzipped tarball named like:

```text
rtsp-viewer-0.1.0-linux-x86_64.tar.gz
```

The tarball contains the compiled `rtsp_viewer_tauri` binary, the project license, and Linux runtime notes.
