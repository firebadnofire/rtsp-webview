# RTSP Viewer (Rust + WebView) — RC1 Spec for Agentic Implementation

**Date:** 2026-02-28  
**Goal:** Produce a cross-platform desktop RTSP viewer (Windows, macOS, Linux) implementing parity with the existing Python/PyQt application while extending the UI model to support multiple 2×2 **Screens** (workspaces). Each Screen contains 4 Panels. The app includes secure credential handling and a clear, typed IPC boundary between UI and backend.

RC1 is a “feature-parity first” release candidate. Performance optimizations beyond basic backpressure and frame dropping are deferred unless required to meet the acceptance criteria.

---

## 0) Non-Negotiable Requirements

1. Cross-platform build and run on:
   - Windows 10/11 (x86_64)
   - macOS 12+ (Intel + Apple Silicon)
   - Linux (x86_64, glibc baseline; Wayland and X11 supported via WebView)
2. App supports multiple **Screens** (workspaces). Each Screen is a fixed **2×2** grid.
3. Each Screen contains exactly **four** Panels.
4. Default Screens count is **4**; configurable by user setting.
5. Per-panel settings editable in UI.
6. Start/stop per panel.
7. Start/stop per screen.
8. Global start/stop across all screens.
9. Fullscreen view for the active panel with quick exit controls.
10. Snapshot capture for an active stream.
11. Save/load configuration.
12. Auto-reconnect with visible state and error reporting.
13. Passwords must **not** be persisted in plaintext config files.
14. Never log credentials or full credentialed URLs.

---

## 1) Product Definition

### 1.1 User-facing features (parity + screens)
- Multiple **Screens** (workspaces), each showing a fixed 2×2 grid of RTSP Panels.
- Default screens: **4** (configurable).
- Each Panel has editable fields:
  - Title
  - Host/IP
  - Port
  - Slug/path
  - Channel
  - Subtype
  - Transport (TCP/UDP)
  - Latency (ms)
  - Username
  - Password (stored in OS credential store)
- Navigation:
  - Screen switcher (tabs recommended for RC1)
  - Active screen clearly highlighted
- Controls:
  - Start/Stop per panel
  - Start Screen / Stop Screen (acts on active screen)
  - Start All Cameras / Stop All Cameras (acts across all screens)
  - Select active panel by click (scoped to active screen)
  - Fullscreen toggle for active panel
  - Snapshot button for active panel
  - Save Config / Load Config
- URL preview behavior:
  - Preview must render a usable RTSP URL, but **must not** reveal the password by default.
  - Provide a UI toggle “Reveal password in preview” (default OFF).

### 1.2 Non-goals for RC1
- More than 4 panels.
- ONVIF discovery.
- Cloud sync.
- Multi-user accounts.
- Advanced GPU texture bridge.
- Advanced recording, timeline playback.

---

## 2) Technical Approach Summary

### 2.1 Stack
- Backend: Rust stable + Tokio
- Desktop shell: Tauri (Wry WebView)
- Frontend: TypeScript + HTML/CSS (framework optional; prefer minimal)
- Serialization: Serde
- Errors: thiserror
- Logging: tracing (with strict redaction)
- Secrets:
  - Primary: OS credential store via `keyring` crate (Windows Credential Manager, macOS Keychain, Linux Secret Service)
  - Optional fallback: local encrypted secrets file using Argon2id + XChaCha20-Poly1305

### 2.2 Media backend strategy
RC1 must implement one working backend behind a trait.

**RC1 default backend:** FFmpeg-based decode.
- Preferred crates: `ffmpeg-next` or `rsmpeg`.
- Abstraction required so that a future GStreamer backend can be added without changing UI/IPC.

**Important packaging constraint:** FFmpeg libraries may not be available by default on end-user systems. RC1 must provide a documented and automated strategy to produce runnable binaries on all 3 OSes.
- Windows: vcpkg or bundled FFmpeg DLLs placed in app directory.
- macOS: bundle dylibs or use static where feasible.
- Linux: AppImage can bundle; otherwise document system dependency.

If the agent cannot reliably bundle FFmpeg for all platforms within RC1 scope, it must still deliver a working development build path on each OS, and clearly mark release packaging as RC2.

---

## 3) Repository Layout

Use a Cargo workspace and a Tauri app.

```
repo/
  src-tauri/
    Cargo.toml
    src/main.rs
    build.rs (only if required)
  crates/
    core/
    media/
    config/
    secrets/
  ui/
    package.json
    src/
    index.html
```

### 3.1 Crate responsibilities
- `crates/core`
  - Domain types: panel IDs, state machine, IPC structs shared by backend modules
  - URL building and redaction utilities
- `crates/config`
  - Config schema structs, validation, atomic save/load, migrations
- `crates/secrets`
  - Secret store abstraction, keyring implementation, optional encrypted fallback
- `crates/media`
  - Stream worker, decoder, reconnect strategy, frame publishing, snapshot capture
- `src-tauri`
  - IPC command handlers, event emission, app bootstrap, file dialogs

---

## 4) UI Requirements (WebView Frontend)

### 4.1 Screen navigation
- Provide a screen switcher UI.
- RC1 recommended: tab bar labeled `Screen 1`, `Screen 2`, ...
- Switching screens must not interrupt running streams.
- Minimum: click to select active screen.

### 4.2 Layout
- Active Screen displays a fixed 2×2 grid of video panes.
- Each pane includes:
  - Title label
  - Compact status line (state + short message)
  - Start/Stop button
  - Snapshot button (enabled when Playing)
  - Settings button (opens panel settings drawer/modal)
- Global controls:
  - Start Screen / Stop Screen (for active screen)
  - Start All Cameras / Stop All Cameras (global)
  - Save Config / Load Config
  - Fullscreen toggle for active panel

### 4.3 Active screen/panel behavior
- Clicking a pane sets it active within the active screen.
- Active pane has visible highlight.
- Fullscreen always shows the active panel of the active screen.
- Keyboard exits fullscreen: Esc, F11, Q.

### 4.4 Frontend code conventions
- TypeScript only.
- **Do not include comments in JS/TS files.**
- TypeScript only.
- **Do not include comments in JS/TS files.**

---

## 5) IPC Contract (Typed)

All IPC messages must be versioned and strictly typed.

### 5.1 Common types
- `screen_id`: integer in range 0.. (dense, starting at 0)
- `panel_id`: integer in range 0..=3
- `ipc_version`: string, example `"1"`

### 5.2 UI -> Rust Commands
Implement as Tauri commands.

1. `set_active_screen(screen_id)`
2. `set_active_panel(screen_id, panel_id)`
3. `get_state()`
   - returns current app state: active screen, active panel per screen, per-panel config (non-secrets), per-panel status
4. `update_panel_config(screen_id, panel_id, patch)`
   - patch is a partial object with validated fields
5. `set_panel_secret(screen_id, panel_id, username, password)`
   - stores in secret store
6. `start_stream(screen_id, panel_id)`
7. `stop_stream(screen_id, panel_id)`
8. `start_screen(screen_id)`
9. `stop_screen(screen_id)`
10. `start_all_global()`
11. `stop_all_global()`
12. `save_config(path | null)`
   - if null, open save dialog
13. `load_config(path | null)`
   - if null, open open dialog
14. `snapshot(screen_id, panel_id, path | null)`
   - if null, open save dialog; default filename includes timestamp
15. `toggle_fullscreen(enabled)`

Optional for RC1 (implement if straightforward):
- `create_screen()`
- `delete_screen(screen_id)` (must stop all streams in that screen first)

### 5.3 Rust -> UI Events
Emit via Tauri events.

1. `panel_status`
   - payload: `{ screen_id, panel_id, state, message, code? }`
2. `panel_frame`
   - payload: `{ screen_id, panel_id, mime, data_base64, width?, height?, pts_ms? }`
   - RC1 uses JPEG frames: `mime = "image/jpeg"`
3. `config_loaded`
   - payload: sanitized config + secret presence flags per panel
4. `snapshot_saved` / `snapshot_failed`
   - payload includes `{ screen_id, panel_id, ... }`
5. `security_notice`
   - payload: `{ code, message }`

### 5.4 Event rate limits
- `panel_status`: at most 5 per second per panel.
- `panel_frame`: bounded by a target FPS (default 10 FPS) and a bounded queue.
- `panel_status`: at most 5 per second per panel.
- `panel_frame`: bounded by a target FPS (default 10 FPS) and a bounded queue.

---

## 6) Rendering Path (RC1)

### 6.1 Frame transport (RC1)
- Backend decodes frames to RGB/RGBA.
- Backend encodes to JPEG with configurable quality (default 80).
- Backend emits `panel_frame` events with base64 payload.
- Frontend updates an `<img>` element per panel using `data:` URL.

### 6.2 Backpressure policy
- For each panel, maintain a bounded queue for outgoing frames.
- If full, drop the oldest frame and enqueue the newest.
- UI should not attempt to render every frame, it should just display the latest.

### 6.3 Aspect ratio
- Preserve aspect ratio.
- Fit-to-pane with letterboxing if needed.

---

## 7) Media Pipeline (Backend)

### 7.1 Stream options
Per panel, the effective runtime options are:
- RTSP URL parts: host, port, path/slug
- Username and password resolved from secret store
- Transport: TCP/UDP
- Latency: ms (affects buffer size and timeouts)

### 7.2 State machine
Per panel:
- `Idle`
- `Connecting`
- `Playing`
- `Retrying`
- `Error`
- `Stopped`

Rules:
- `start_stream` from `Idle|Stopped|Error` transitions to `Connecting`.
- `stop_stream` transitions to `Stopped` and cancels all tasks.
- Decode loop failures transition to `Retrying` unless stop requested.
- `Retrying` uses bounded exponential backoff with jitter.

### 7.3 Concurrency model
- A central `StreamManager` owns all 4 panel supervisors.
- Each panel supervisor:
  - has a `CancellationToken`
  - has a control channel for start/stop and config updates
  - runs decode pipeline in a Tokio task

### 7.4 Reconnect policy
- Base delay: 500ms
- Exponential factor: 2.0
- Max delay: 10s
- Jitter: random 0..250ms
- Max consecutive failures before `Error`: 30 (still allow manual Start to retry)

### 7.5 Timeouts
- Connection timeout: 5s
- Read/decode stall timeout: 5s triggers reconnect

All timeouts must be configurable in a hidden “advanced” config section for RC1, but defaults must be conservative.

---

## 8) Configuration and Persistence

### 8.1 Config file format
- JSON
- Versioned schema
- Atomic writes: write temp, fsync, rename

### 8.2 Schema (RC1)
Top-level:
- `schema_version: u32` (start at 2)
- `screens: Vec<ScreenConfig>`
- `ui_state: UiState`

`ScreenConfig`:
- `id: u32` (dense starting at 0)
- `panels: [PanelConfig; 4]`

`PanelConfig` (non-secret fields):
- `title: String`
- `host: String`
- `port: u16`
- `path: String`
- `channel: Option<String>`
- `subtype: Option<String>`
- `transport: "tcp" | "udp"`
- `latency_ms: u32`
- `secret_ref: SecretRef`

`SecretRef`:
- `key: String` (stable identifier for looking up secrets)

`UiState`:
- `active_screen: u32`
- `active_panel_per_screen: Vec<u8>` (indexed by screen order; values 0..=3)
- `fullscreen: bool`
- Optional: window size/position if feasible in Tauri

### 8.3 Validation
- `screens.len()` bound: 1..=32 (RC1 recommended; enforce to prevent runaway resource usage)
- Each screen must have exactly 4 panels.
- `host` must be non-empty to start.
- `port` in 1..=65535
- `latency_ms` bound: 0..=5000
- `active_screen` must exist
- `active_panel_per_screen[i]` in 0..=3

Invalid config load must:
- reject with a user-safe error message
- not crash

### 8.4 Save/load UX
- Save/load should use native file dialogs by default.
- Load replaces runtime config and updates UI.
- Streams must not stop merely because a screen becomes inactive.
- On load, for panels currently Playing:
  - restart only if the connection tuple changed (host/port/path/transport/credentials presence).
  - otherwise keep running.
- Save/load should use native file dialogs by default.
- Load merges into runtime state and triggers panel restarts only if the panel is currently Playing and the connection tuple changed.

---

## 9) Secrets Handling

### 9.1 Primary path: OS credential store
- Use `keyring` crate.
- Store per panel using `secret_ref.key`.
- Store username and password.

### 9.2 Secret key derivation
- Default pattern: `"screen_{screen_id}_panel_{panel_id}"`.
- Keys must be stable across saves/loads as long as screen_id/panel_id remain stable.

### 9.3 Secret presence reporting
- UI must show whether credentials are present for a panel without revealing them.

### 9.4 Optional fallback: encrypted secrets file
Only if keyring is unavailable.
- Derive key with Argon2id.
- Encrypt with XChaCha20-Poly1305.
- Prompt for a master password.
- This path can be deferred to RC2 if keyring works on all 3 OSes.

### 9.5 Redaction rules
Never expose in:
- logs
- status messages
- panic traces
- IPC payloads

URLs must be redacted as `rtsp://user:***@host:port/path`.
Never expose in:
- logs
- status messages
- panic traces
- IPC payloads

URLs must be redacted as `rtsp://user:***@host:port/path`.

---

## 10) Error Handling and Observability

### 10.1 Error taxonomy
Define a backend error enum with stable codes, for example:
- `E_CONFIG_INVALID`
- `E_SECRET_MISSING`
- `E_RTSP_AUTH`
- `E_RTSP_CONNECT`
- `E_DECODE`
- `E_TIMEOUT`
- `E_IO`

Map each to:
- user-safe message
- internal diagnostic context (redacted)

### 10.2 Logging
- Use `tracing`.
- Default log level: info.
- Debug logs must remain redacted.

---

## 11) Platform Packaging

### 11.1 Build requirements
- Rust stable toolchain
- Node.js LTS for frontend
- Tauri toolchain prerequisites per OS

### 11.2 Packaging deliverables
RC1 must produce:
- A runnable dev build on all 3 OSes.
- At least one distributable artifact format, preferably:
  - Windows: MSI or NSIS
  - macOS: .app bundle (optionally dmg)
  - Linux: AppImage

If FFmpeg bundling blocks this, the agent must still output reproducible build docs and CI scripts that produce dev builds.

---

## 12) Testing

### 12.1 Unit tests
- Config validation boundaries
- URL building and redaction
- Secret store mock behavior
- State transitions under start/stop/retry

### 12.2 Integration tests
- Provide a test harness that can run at least one RTSP source.
- Preferred approach:
  - Use a bundled test mode that plays an embedded sample or test pattern without network.
  - If not feasible, document use of an external RTSP test server.

### 12.3 Manual test checklist
- Start all 4 panels (with valid endpoints)
- Stop one panel while others continue
- Fullscreen active panel and exit via Esc/F11/Q
- Snapshot saves correctly
- Load config and credentials remain present
- Reconnect occurs on simulated network drop
- No credential appears in logs or UI status

---

## 13) Acceptance Criteria (RC1)

RC1 is accepted when all are true:

1. Multiple Screens are supported; each Screen displays a 2×2 grid.
2. Default screen count is 4 and is configurable.
3. Switching screens does not interrupt streams.
4. Each panel can connect to an RTSP stream and display motion video.
5. Start/stop per panel works without orphan tasks.
6. Start Screen / Stop Screen affects only the specified screen.
7. Global Start All Cameras starts all screens without requiring UI navigation; Global Stop stops all.
8. Fullscreen behavior works and exits reliably.
9. Snapshot produces an image file from the current stream.
10. Config save/load works with schema versioning and atomic writes.
11. Secrets are stored outside config files and are recoverable across restarts.
12. Auto-reconnect works with bounded backoff and visible status.
13. No plaintext credentials in logs, config, IPC, or crash output.
14. Builds run on Windows, macOS, Linux.

---

## 14) Implementation Plan (Order of Execution)

1. Create workspace structure and compile empty Tauri app.
2. Implement `core` types (screen/panel IDs, states, IPC payload types).
3. Implement `config` crate (schema v2 with screens, validation, atomic save/load).
4. Implement `secrets` crate using keyring + mock.
5. Implement `media` abstraction:
   - `StreamManager` keyed by (screen_id, panel_id)
   - per-panel supervisor with cancellation
   - reconnect policy
   - decode loop
6. Implement RC1 frame publisher:
   - encode to JPEG
   - emit `panel_frame` with screen_id/panel_id
7. Implement Tauri commands and event emission:
   - start/stop stream
   - start/stop screen
   - global start/stop
8. Implement frontend UI:
   - screen switcher
   - per-screen active panel
   - screen/global controls
9. Add tests and a test harness.
10. Cross-platform packaging scripts and docs.

---

## 15) Deliverables

- Source repository with:
  - Cargo workspace
  - Tauri app
  - UI code
  - Build documentation for Windows/macOS/Linux
- RC1 artifacts or reproducible steps to generate them
- A short SECURITY.md describing redaction and secret storage behavior

---

## 16) Agent Constraints and Quality Gates

- No credentials in any logged output.
- Use bounded channels and explicit cancellation for all streaming tasks.
- Avoid shared mutable state across tasks; prefer message passing.
- Run `cargo fmt` and `cargo clippy` clean.
- Frontend TS build must be reproducible.
- Do not add JS/TS comments.

