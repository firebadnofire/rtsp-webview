# rtsp-webview

## Makefile usage

```bash
make setup
```
Install UI dependencies (`npm ci` in `ui/`).

```bash
make ui-build
```
Build the frontend bundle used by Tauri.

```bash
make run
```
Build the frontend bundle and then run the app with `cargo run`.

```bash
make release-bin
```
Build the frontend bundle and produce a release binary at `target/release/rtsp_viewer_tauri`.

```bash
make rust-test
```
Run Rust tests.

```bash
make ui-test
```
Run UI tests.

```bash
make test
```
Run both Rust and UI tests.

```bash
make fmt
```
Format Rust code.

```bash
make clean
```
Remove build and install artifacts (`target/`, `ui/dist`, and `ui/node_modules`).
