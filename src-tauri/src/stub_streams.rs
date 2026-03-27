use crate::app_state::{ManagedState, StreamTask};
use crate::errors::CommandError;
use crate::events;
use crate::state::{FrameCache, PanelKey};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rtsp_core::{PanelFrameEvent, PanelState, PanelStatusEvent, IPC_VERSION};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, ChildStdout, Command};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use url::Url;

const READ_TIMEOUT: Duration = Duration::from_secs(6);
const INITIAL_READ_TIMEOUT: Duration = Duration::from_secs(15);
const RECONNECT_DELAY: Duration = Duration::from_millis(600);
const MAX_PENDING_BYTES: usize = 2 * 1024 * 1024;
const STATUS_MESSAGE_MAX_LEN: usize = 280;
const STARTUP_KEYFRAME_MESSAGE: &str = "Waiting for initial keyframe";
const PREVIEW_MAX_WIDTH: u32 = 2560;
const PREVIEW_MAX_HEIGHT: u32 = 1440;
const PREVIEW_JPEG_QUALITY: u8 = 4;
const FFMPEG_OVERRIDE_ENV: &str = "RTSP_VIEWER_FFMPEG";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const FFMPEG_BINARY_NAMES: &[&str] = &["ffmpeg.exe", "ffmpeg.cmd", "ffmpeg.bat", "ffmpeg"];
#[cfg(not(windows))]
const FFMPEG_BINARY_NAMES: &[&str] = &["ffmpeg"];

pub async fn ensure_started(
    app: AppHandle,
    managed: ManagedState,
    key: PanelKey,
) -> Result<(), CommandError> {
    {
        let streams = managed.inner.streams.lock().await;
        if streams.contains_key(&key) {
            return Ok(());
        }
    }

    let cancel = CancellationToken::new();
    let task_cancel = cancel.clone();
    let task_managed = managed.clone();
    let task_app = app.clone();

    let handle = tokio::spawn(async move {
        let _ = run_loop(task_app, task_managed, key, task_cancel).await;
    });

    let mut streams = managed.inner.streams.lock().await;
    streams.insert(key, StreamTask { cancel, handle });

    Ok(())
}

pub async fn stop_stream(
    app: AppHandle,
    managed: ManagedState,
    key: PanelKey,
    emit_status: bool,
) -> Result<(), CommandError> {
    let task = {
        let mut streams = managed.inner.streams.lock().await;
        streams.remove(&key)
    };

    if let Some(task) = task {
        task.cancel.cancel();
        let _ = timeout(Duration::from_secs(2), task.handle).await;
    }

    {
        let mut runtime = managed.inner.runtime.write().await;
        if runtime.panel_exists(key) {
            runtime.set_panel_status(key, PanelState::Stopped, "Stopped", None)?;
        }
    }

    if emit_status {
        let _ = events::emit_panel_status(
            &app,
            PanelStatusEvent {
                ipc_version: IPC_VERSION.to_string(),
                screen_id: key.screen_id,
                panel_id: key.panel_id,
                state: PanelState::Stopped,
                message: "Stopped".to_string(),
                code: None,
            },
        );
    }

    Ok(())
}

async fn run_loop(
    app: AppHandle,
    managed: ManagedState,
    key: PanelKey,
    cancel: CancellationToken,
) -> Result<(), CommandError> {
    set_status(
        &app,
        managed.clone(),
        key,
        PanelState::Connecting,
        "Connecting",
        None,
    )
    .await?;

    let mut frame_seq: u64 = 0;
    let mut is_playing = false;

    loop {
        if cancel.is_cancelled() {
            break;
        }

        let rtsp_url = match panel_rtsp_url(managed.clone(), key).await {
            Ok(url) => url,
            Err(error) => {
                let _ = set_status(
                    &app,
                    managed.clone(),
                    key,
                    PanelState::Error,
                    error.message.clone(),
                    Some(error.code),
                )
                .await;
                if sleep_or_cancel(&cancel, RECONNECT_DELAY).await {
                    break;
                }
                continue;
            }
        };

        match stream_rtsp_session(
            &app,
            managed.clone(),
            key,
            &cancel,
            &rtsp_url,
            &mut frame_seq,
            &mut is_playing,
        )
        .await
        {
            Ok(()) => break,
            Err(error) => {
                is_playing = false;
                let _ = set_status(
                    &app,
                    managed.clone(),
                    key,
                    PanelState::Retrying,
                    error.message,
                    Some(error.code),
                )
                .await;
            }
        }

        if sleep_or_cancel(&cancel, RECONNECT_DELAY).await {
            break;
        }
    }

    Ok(())
}

async fn stream_rtsp_session(
    app: &AppHandle,
    managed: ManagedState,
    key: PanelKey,
    cancel: &CancellationToken,
    rtsp_url: &str,
    frame_seq: &mut u64,
    is_playing: &mut bool,
) -> Result<(), CommandError> {
    let preview_fps = {
        let runtime = managed.inner.runtime.read().await;
        runtime.effective_preview_fps_for_key(key)?
    };
    let (mut child, mut stdout, stderr_task) = spawn_ffmpeg_process(rtsp_url, preview_fps)?;
    let mut read_buffer = [0u8; 8192];
    let mut pending = Vec::with_capacity(128 * 1024);

    loop {
        let read_result = tokio::select! {
            _ = cancel.cancelled() => {
                terminate_child(&mut child).await;
                let _ = wait_for_exit_code(&mut child).await;
                let _ = collect_stderr(stderr_task).await;
                return Ok(());
            }
            result = timeout(read_timeout_for_stream(*is_playing), stdout.read(&mut read_buffer)) => result,
        };

        let read_count = match read_result {
            Ok(result) => result.map_err(|error| {
                CommandError::decode(format!("ffmpeg stdout read failed: {}", error))
            })?,
            Err(_) => {
                terminate_child(&mut child).await;
                let exit_code = wait_for_exit_code(&mut child).await;
                let stderr = collect_stderr(stderr_task).await;
                return Err(CommandError::decode(format_stream_error(
                    exit_code,
                    &stderr,
                    "ffmpeg did not produce a frame in time",
                    *is_playing,
                )));
            }
        };

        if read_count == 0 {
            let exit_code = wait_for_exit_code(&mut child).await;
            let stderr = collect_stderr(stderr_task).await;
            return Err(CommandError::decode(format_stream_error(
                exit_code,
                &stderr,
                "ffmpeg stream ended before producing a frame",
                *is_playing,
            )));
        }

        pending.extend_from_slice(&read_buffer[..read_count]);

        while let Some(frame_bytes) = extract_jpeg_frame(&mut pending) {
            emit_frame(
                app,
                managed.clone(),
                key,
                frame_bytes,
                frame_seq,
                is_playing,
            )
            .await?;
        }

        prune_pending_buffer(&mut pending);
    }
}

fn spawn_ffmpeg_process(
    rtsp_url: &str,
    preview_fps: u8,
) -> Result<(Child, ChildStdout, JoinHandle<String>), CommandError> {
    let preview_filter = build_preview_filter(preview_fps);
    let jpeg_quality = PREVIEW_JPEG_QUALITY;
    let ffmpeg_executable = resolve_ffmpeg_executable()?;
    let mut command = Command::new(&ffmpeg_executable);
    command
        .arg("-nostdin")
        .arg("-v")
        .arg("error")
        .arg("-rtsp_transport")
        .arg("tcp")
        .arg("-rtsp_flags")
        .arg("prefer_tcp")
        .arg("-timeout")
        .arg("3000000")
        .arg("-analyzeduration")
        .arg("0")
        .arg("-probesize")
        .arg("32768")
        .arg("-fflags")
        .arg("discardcorrupt")
        .arg("-i")
        .arg(rtsp_url)
        .arg("-f")
        .arg("image2pipe")
        .arg("-vf")
        .arg(preview_filter)
        .arg("-vcodec")
        .arg("mjpeg")
        .arg("-q:v")
        .arg(jpeg_quality.to_string())
        .arg("-")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    configure_background_process(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| map_ffmpeg_spawn_error(&ffmpeg_executable, error))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CommandError::decode("ffmpeg stdout unavailable"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| CommandError::decode("ffmpeg stderr unavailable"))?;

    let stderr_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes).await;
        String::from_utf8_lossy(&bytes).trim().to_string()
    });

    Ok((child, stdout, stderr_task))
}

fn resolve_ffmpeg_executable() -> Result<PathBuf, CommandError> {
    if let Some(configured) =
        std::env::var_os(FFMPEG_OVERRIDE_ENV).filter(|value| !value.is_empty())
    {
        let configured_path = PathBuf::from(configured);
        if is_path_like(&configured_path) && !configured_path.is_file() {
            return Err(CommandError::decode(format!(
                "{} points to a missing file: {}",
                FFMPEG_OVERRIDE_ENV,
                configured_path.display()
            )));
        }
        return Ok(configured_path);
    }

    let search_dirs = ffmpeg_search_dirs();
    resolve_executable_in_dirs(&search_dirs, FFMPEG_BINARY_NAMES)
        .ok_or_else(|| CommandError::decode(ffmpeg_not_found_message()))
}

fn is_path_like(path: &Path) -> bool {
    path.is_absolute() || path.components().count() > 1
}

fn ffmpeg_search_dirs() -> Vec<PathBuf> {
    let mut search_dirs = Vec::new();

    append_path_search_dirs(&mut search_dirs);

    let current_exe = std::env::current_exe().ok();
    append_executable_relative_search_dirs(&mut search_dirs, current_exe.as_deref());
    append_platform_ffmpeg_search_dirs(&mut search_dirs);

    search_dirs
}

fn append_path_search_dirs(search_dirs: &mut Vec<PathBuf>) {
    let Some(path_value) = std::env::var_os("PATH") else {
        return;
    };

    for directory in std::env::split_paths(&path_value) {
        if !directory.as_os_str().is_empty() {
            push_unique_search_dir(search_dirs, directory);
        }
    }
}

fn append_executable_relative_search_dirs(
    search_dirs: &mut Vec<PathBuf>,
    current_exe: Option<&Path>,
) {
    let Some(current_exe) = current_exe else {
        return;
    };

    let Some(executable_dir) = current_exe.parent() else {
        return;
    };

    push_unique_search_dir(search_dirs, executable_dir.to_path_buf());
    push_unique_search_dir(search_dirs, executable_dir.join("bin"));

    #[cfg(target_os = "macos")]
    if let Some(contents_dir) = executable_dir.parent() {
        push_unique_search_dir(search_dirs, contents_dir.join("Resources"));
        push_unique_search_dir(search_dirs, contents_dir.join("Resources").join("bin"));
    }
}

#[cfg(target_os = "macos")]
fn append_platform_ffmpeg_search_dirs(search_dirs: &mut Vec<PathBuf>) {
    for directory in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/opt/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        push_unique_search_dir(search_dirs, PathBuf::from(directory));
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn append_platform_ffmpeg_search_dirs(search_dirs: &mut Vec<PathBuf>) {
    for directory in ["/usr/local/bin", "/usr/bin", "/bin", "/snap/bin"] {
        push_unique_search_dir(search_dirs, PathBuf::from(directory));
    }
}

#[cfg(windows)]
fn append_platform_ffmpeg_search_dirs(search_dirs: &mut Vec<PathBuf>) {
    if let Some(program_files) = std::env::var_os("ProgramFiles").filter(|value| !value.is_empty())
    {
        push_unique_search_dir(
            search_dirs,
            PathBuf::from(program_files).join("ffmpeg").join("bin"),
        );
    }
    if let Some(program_files_x86) =
        std::env::var_os("ProgramFiles(x86)").filter(|value| !value.is_empty())
    {
        push_unique_search_dir(
            search_dirs,
            PathBuf::from(program_files_x86).join("ffmpeg").join("bin"),
        );
    }
}

fn push_unique_search_dir(search_dirs: &mut Vec<PathBuf>, directory: PathBuf) {
    if !search_dirs.iter().any(|candidate| candidate == &directory) {
        search_dirs.push(directory);
    }
}

fn resolve_executable_in_dirs(search_dirs: &[PathBuf], binary_names: &[&str]) -> Option<PathBuf> {
    for directory in search_dirs {
        for binary_name in binary_names {
            let candidate = directory.join(binary_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn map_ffmpeg_spawn_error(ffmpeg_executable: &Path, error: std::io::Error) -> CommandError {
    if error.kind() == ErrorKind::NotFound {
        return CommandError::decode(ffmpeg_not_found_message());
    }

    CommandError::decode(format!(
        "failed to run ffmpeg at {}: {}",
        ffmpeg_executable.display(),
        error
    ))
}

#[cfg(target_os = "macos")]
fn ffmpeg_not_found_message() -> String {
    "ffmpeg was not found on PATH or in common macOS locations. Finder-launched apps do not inherit your shell PATH; install ffmpeg in /opt/homebrew/bin or /usr/local/bin, or set RTSP_VIEWER_FFMPEG to an absolute path.".to_string()
}

#[cfg(not(target_os = "macos"))]
fn ffmpeg_not_found_message() -> String {
    "ffmpeg was not found on PATH or in common install locations. Set RTSP_VIEWER_FFMPEG to an absolute path if it is installed elsewhere.".to_string()
}

#[cfg(windows)]
fn configure_background_process(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_background_process(_command: &mut Command) {}

fn build_preview_filter(preview_fps: u8) -> String {
    format!(
        "fps={preview_fps},scale=w='min({max_width},iw)':h='min({max_height},ih)':force_original_aspect_ratio=decrease:flags=fast_bilinear",
        max_width = PREVIEW_MAX_WIDTH,
        max_height = PREVIEW_MAX_HEIGHT,
    )
}

async fn terminate_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.start_kill();
    }
}

async fn wait_for_exit_code(child: &mut Child) -> Option<i32> {
    match timeout(Duration::from_secs(2), child.wait()).await {
        Ok(Ok(status)) => status.code(),
        _ => None,
    }
}

async fn collect_stderr(task: JoinHandle<String>) -> String {
    task.await.unwrap_or_default()
}

fn read_timeout_for_stream(is_playing: bool) -> Duration {
    if is_playing {
        READ_TIMEOUT
    } else {
        INITIAL_READ_TIMEOUT
    }
}

fn format_stream_error(
    exit_code: Option<i32>,
    stderr: &str,
    fallback: &str,
    is_playing: bool,
) -> String {
    if !is_playing && is_transient_h264_startup_error(stderr) {
        return STARTUP_KEYFRAME_MESSAGE.to_string();
    }

    format_ffmpeg_error(exit_code, stderr, fallback)
}

fn format_ffmpeg_error(exit_code: Option<i32>, stderr: &str, fallback: &str) -> String {
    let detail = truncate_status(stderr);
    if !detail.is_empty() {
        return format!("ffmpeg: {}", detail);
    }

    match exit_code {
        Some(code) => format!("{} (exit code {})", fallback, code),
        None => fallback.to_string(),
    }
}

fn truncate_status(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= STATUS_MESSAGE_MAX_LEN {
        return normalized;
    }

    let mut output = normalized
        .chars()
        .take(STATUS_MESSAGE_MAX_LEN.saturating_sub(3))
        .collect::<String>();
    output.push_str("...");
    output
}

fn is_transient_h264_startup_error(stderr: &str) -> bool {
    let normalized = stderr.to_ascii_lowercase();
    normalized.contains("co located pocs unavailable")
        || normalized.contains("mmco: unref short failure")
        || normalized.contains("reference picture missing during reorder")
        || normalized.contains("missing picture in access unit")
        || normalized.contains("decode_slice_header error")
        || normalized.contains("non-existing pps 0 referenced")
        || normalized.contains("no frame!")
}

async fn sleep_or_cancel(cancel: &CancellationToken, duration: Duration) -> bool {
    tokio::select! {
        _ = cancel.cancelled() => true,
        _ = sleep(duration) => false,
    }
}

async fn panel_rtsp_url(managed: ManagedState, key: PanelKey) -> Result<String, CommandError> {
    let (host, port, path, channel, subtype, secret) = {
        let runtime = managed.inner.runtime.read().await;
        let panel = runtime.get_panel(key)?;
        (
            panel.config.host.clone(),
            panel.config.port,
            panel.config.path.clone(),
            panel.config.channel.clone(),
            panel.config.subtype.clone(),
            panel.secret.clone(),
        )
    };

    let host = host.trim().to_string();
    if host.is_empty() {
        return Err(CommandError::config("Host must be configured"));
    }

    let mut full_path = path.trim_start_matches('/').to_string();
    if let Some(channel) = channel {
        let channel = channel.trim();
        if !channel.is_empty() {
            if !full_path.is_empty() {
                full_path.push('/');
            }
            full_path.push_str(channel);
        }
    }
    if let Some(subtype) = subtype {
        let subtype = subtype.trim();
        if !subtype.is_empty() {
            if !full_path.is_empty() {
                full_path.push('/');
            }
            full_path.push_str(subtype);
        }
    }

    let mut parsed = Url::parse(&format!("rtsp://{}:{}/{}", host, port, full_path))
        .map_err(|error| CommandError::config(format!("invalid RTSP URL: {}", error)))?;

    if let Some(secret) = secret {
        if !secret.username.trim().is_empty() {
            parsed
                .set_username(secret.username.trim())
                .map_err(|_| CommandError::config("invalid username in credentials"))?;
            if !secret.password.is_empty() {
                parsed
                    .set_password(Some(&secret.password))
                    .map_err(|_| CommandError::config("invalid password in credentials"))?;
            }
        }
    }

    Ok(parsed.to_string())
}

async fn emit_frame(
    app: &AppHandle,
    managed: ManagedState,
    key: PanelKey,
    frame_bytes: Vec<u8>,
    frame_seq: &mut u64,
    is_playing: &mut bool,
) -> Result<(), CommandError> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    if *frame_seq >= now_ms {
        *frame_seq = frame_seq.saturating_add(1);
    } else {
        *frame_seq = now_ms;
    }
    let frame_base64 = STANDARD.encode(frame_bytes);
    let cached_frame = FrameCache {
        mime: "image/jpeg".to_string(),
        data_base64: frame_base64.clone(),
        width: None,
        height: None,
        pts_ms: Some(now_ms),
        seq: *frame_seq,
    };
    let should_emit = {
        let mut runtime = managed.inner.runtime.write().await;
        if !runtime.panel_exists(key) {
            return Ok(());
        }
        runtime.set_latest_frame(key, cached_frame.clone())?;
        key.screen_id == runtime.active_screen
    };

    if !*is_playing {
        set_status(
            app,
            managed.clone(),
            key,
            PanelState::Playing,
            "Playing",
            None,
        )
        .await?;
        *is_playing = true;
    }

    if should_emit {
        events::emit_panel_frame(
            app,
            PanelFrameEvent {
                ipc_version: IPC_VERSION.to_string(),
                screen_id: key.screen_id,
                panel_id: key.panel_id,
                mime: cached_frame.mime,
                data_base64: frame_base64,
                width: cached_frame.width,
                height: cached_frame.height,
                pts_ms: cached_frame.pts_ms,
                seq: cached_frame.seq,
            },
        )?;
    }

    Ok(())
}

fn extract_jpeg_frame(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let start = buffer
        .windows(2)
        .position(|window| window == [0xFF, 0xD8])?;
    if start > 0 {
        buffer.drain(..start);
    }

    let end_rel = buffer[2..]
        .windows(2)
        .position(|window| window == [0xFF, 0xD9])?;
    let end = end_rel + 4;
    let frame = buffer[..end].to_vec();
    buffer.drain(..end);
    Some(frame)
}

fn prune_pending_buffer(buffer: &mut Vec<u8>) {
    if buffer.len() <= MAX_PENDING_BYTES {
        return;
    }

    if let Some(last_soi) = buffer.windows(2).rposition(|window| window == [0xFF, 0xD8]) {
        buffer.drain(..last_soi);
    } else {
        buffer.clear();
    }
}

async fn set_status(
    app: &AppHandle,
    managed: ManagedState,
    key: PanelKey,
    state: PanelState,
    message: impl Into<String>,
    code: Option<String>,
) -> Result<(), CommandError> {
    let message = message.into();
    {
        let mut runtime = managed.inner.runtime.write().await;
        if !runtime.panel_exists(key) {
            return Ok(());
        }
        runtime.set_panel_status(key, state, message.clone(), code.clone())?;
    }

    events::emit_panel_status(
        app,
        PanelStatusEvent {
            ipc_version: IPC_VERSION.to_string(),
            screen_id: key.screen_id,
            panel_id: key.panel_id,
            state,
            message,
            code,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn startup_reads_allow_more_time_than_steady_state() {
        assert_eq!(read_timeout_for_stream(false), INITIAL_READ_TIMEOUT);
        assert_eq!(read_timeout_for_stream(true), READ_TIMEOUT);
    }

    #[test]
    fn transient_h264_startup_errors_are_normalized() {
        let stderr =
            "[h264 @ 0x1] co located POCs unavailable [h264 @ 0x2] mmco: unref short failure";
        assert_eq!(
            format_stream_error(None, stderr, "ffmpeg failed", false),
            STARTUP_KEYFRAME_MESSAGE
        );
    }

    #[test]
    fn authentication_errors_keep_original_detail() {
        let message = format_stream_error(
            None,
            "Server returned 401 Unauthorized (authorization failed)",
            "ffmpeg failed",
            false,
        );
        assert_eq!(
            message,
            "ffmpeg: Server returned 401 Unauthorized (authorization failed)"
        );
    }

    #[test]
    fn preview_filter_uses_single_fullscreen_cap_for_all_layouts() {
        let filter = build_preview_filter(7);
        assert_eq!(
            filter,
            "fps=7,scale=w='min(2560,iw)':h='min(1440,ih)':force_original_aspect_ratio=decrease:flags=fast_bilinear"
        );
        assert_eq!(PREVIEW_JPEG_QUALITY, 4);
    }

    #[test]
    fn executable_resolution_uses_search_directories() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let missing_dir = temp_dir.path().join("missing");
        let present_dir = temp_dir.path().join("present");
        fs::create_dir_all(&missing_dir).expect("missing dir should create");
        fs::create_dir_all(&present_dir).expect("present dir should create");

        let executable_name = FFMPEG_BINARY_NAMES[0];
        let executable_path = present_dir.join(executable_name);
        fs::write(&executable_path, b"stub").expect("stub executable should write");

        let resolved = resolve_executable_in_dirs(&[missing_dir, present_dir], FFMPEG_BINARY_NAMES)
            .expect("executable should resolve");

        assert_eq!(resolved, executable_path);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn executable_relative_search_dirs_include_app_bundle_resources() {
        let current_exe =
            PathBuf::from("/Applications/RTSP Viewer.app/Contents/MacOS/rtsp_viewer_tauri");
        let mut search_dirs = Vec::new();

        append_executable_relative_search_dirs(&mut search_dirs, Some(&current_exe));

        assert_eq!(
            search_dirs,
            vec![
                PathBuf::from("/Applications/RTSP Viewer.app/Contents/MacOS"),
                PathBuf::from("/Applications/RTSP Viewer.app/Contents/MacOS/bin"),
                PathBuf::from("/Applications/RTSP Viewer.app/Contents/Resources"),
                PathBuf::from("/Applications/RTSP Viewer.app/Contents/Resources/bin"),
            ]
        );
    }
}
