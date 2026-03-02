use crate::app_state::ManagedState;
use crate::errors::CommandError;
use crate::state::{PanelKey, PanelSecret};
use crate::{events, stub_streams};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Local;
use rtsp_core::{
    validate_auto_populate_tool, AppConfig, AutoPopulateTool, ConfigLoadedEvent, GetStateResponse,
    PanelConfigPatch, PanelRuntimeStatus, PanelState, PanelStatusEvent, SecurityNoticeEvent,
    SnapshotFailedEvent, SnapshotSavedEvent, IPC_VERSION,
};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::api::dialog::blocking::FileDialogBuilder;
use tauri::{AppHandle, Manager, State};
use url::Url;

fn resolve_save_path(path: Option<String>, default_name: &str) -> Result<PathBuf, CommandError> {
    if let Some(path) = path {
        return Ok(PathBuf::from(path));
    }

    FileDialogBuilder::new()
        .set_file_name(default_name)
        .save_file()
        .ok_or_else(|| CommandError::io("save was canceled"))
}

fn resolve_open_path(path: Option<String>) -> Result<PathBuf, CommandError> {
    if let Some(path) = path {
        return Ok(PathBuf::from(path));
    }

    FileDialogBuilder::new()
        .pick_file()
        .ok_or_else(|| CommandError::io("open was canceled"))
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), CommandError> {
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::io("invalid save path"))?;

    std::fs::create_dir_all(parent)?;

    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");
    let temp_name = format!(".{}.{}.tmp", stem, std::process::id());
    let temp_path = parent.join(temp_name);

    {
        let mut file = File::create(&temp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
    }

    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&temp_path, path)?;

    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }

    Ok(())
}

fn replace_token(input: &str, token: &str, value: &str) -> String {
    input.replace(token, value)
}

fn resolve_auto_populated_url(tool: &AutoPopulateTool, camera_num: u32, sub_num: u32) -> String {
    let mut output = tool.base_url_template.clone();
    output = replace_token(&output, "$cameraNum", &camera_num.to_string());
    output = replace_token(&output, "$subNum", &sub_num.to_string());
    output = replace_token(&output, "$USERNAME", &tool.username);
    output = replace_token(&output, "$PASSWORD", &tool.password);
    output = replace_token(&output, "$IP", &tool.ip);
    output = replace_token(&output, "$PORT", &tool.port);
    output
}

struct ParsedRtsp {
    host: String,
    port: u16,
    path: String,
    username: String,
    password: String,
}

fn parse_rtsp_url(value: &str) -> Result<ParsedRtsp, CommandError> {
    let parsed = Url::parse(value).map_err(|error| {
        CommandError::config(format!("invalid RTSP url '{}': {}", value, error))
    })?;

    if parsed.scheme() != "rtsp" {
        return Err(CommandError::config(
            "auto-populate URL must use rtsp scheme",
        ));
    }

    let host = parsed
        .host_str()
        .map(ToString::to_string)
        .ok_or_else(|| CommandError::config("auto-populate URL must include host"))?;

    let port = parsed.port().unwrap_or(554);
    let mut path = parsed.path().trim_start_matches('/').to_string();
    if let Some(query) = parsed.query() {
        if !query.is_empty() {
            if !path.is_empty() {
                path.push('?');
                path.push_str(query);
            } else {
                path = format!("?{}", query);
            }
        }
    }

    let username = parsed.username().to_string();
    let password = parsed.password().unwrap_or_default().to_string();

    Ok(ParsedRtsp {
        host,
        port,
        path,
        username,
        password,
    })
}

async fn emit_security_notice(
    app: &AppHandle,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Result<(), CommandError> {
    events::emit_security_notice(
        app,
        SecurityNoticeEvent {
            ipc_version: IPC_VERSION.to_string(),
            code: code.into(),
            message: message.into(),
        },
    )
}

async fn start_panel(
    app: &AppHandle,
    managed: ManagedState,
    key: PanelKey,
) -> Result<(), CommandError> {
    {
        let mut runtime = managed.inner.runtime.write().await;
        let panel = runtime.get_panel_mut(key)?;
        if panel.config.host.trim().is_empty() {
            panel.status.state = PanelState::Error;
            panel.status.message = "Host must be configured".to_string();
            panel.status.code = Some("E_CONFIG_INVALID".to_string());
            drop(runtime);
            events::emit_panel_status(
                app,
                PanelStatusEvent {
                    ipc_version: IPC_VERSION.to_string(),
                    screen_id: key.screen_id,
                    panel_id: key.panel_id,
                    state: PanelState::Error,
                    message: "Host must be configured".to_string(),
                    code: Some("E_CONFIG_INVALID".to_string()),
                },
            )?;
            return Err(CommandError::config("host must be non-empty to start"));
        }

        if matches!(
            panel.status.state,
            PanelState::Playing | PanelState::Connecting
        ) {
            return Ok(());
        }

        panel.status.state = PanelState::Connecting;
        panel.status.message = "Connecting".to_string();
        panel.status.code = None;
    }

    events::emit_panel_status(
        app,
        PanelStatusEvent {
            ipc_version: IPC_VERSION.to_string(),
            screen_id: key.screen_id,
            panel_id: key.panel_id,
            state: PanelState::Connecting,
            message: "Connecting".to_string(),
            code: None,
        },
    )?;

    stub_streams::ensure_started(app.clone(), managed, key).await?;
    Ok(())
}

async fn stop_panel(
    app: &AppHandle,
    managed: ManagedState,
    key: PanelKey,
    emit_status: bool,
) -> Result<(), CommandError> {
    {
        let mut runtime = managed.inner.runtime.write().await;
        if runtime.panel_exists(key) {
            runtime.set_recording(key, false)?;
        }
    }
    stub_streams::stop_stream(app.clone(), managed, key, emit_status).await
}

async fn restart_panel(
    app: &AppHandle,
    managed: ManagedState,
    key: PanelKey,
) -> Result<(), CommandError> {
    stop_panel(app, managed.clone(), key, false).await?;
    start_panel(app, managed, key).await
}

#[tauri::command]
pub async fn get_state(state: State<'_, ManagedState>) -> Result<GetStateResponse, CommandError> {
    let runtime = state.inner.runtime.read().await;
    Ok(runtime.snapshot())
}

#[tauri::command]
pub async fn set_active_screen(
    state: State<'_, ManagedState>,
    screen_id: u32,
) -> Result<(), CommandError> {
    let mut runtime = state.inner.runtime.write().await;
    runtime.set_active_screen(screen_id)
}

#[tauri::command]
pub async fn set_active_panel(
    state: State<'_, ManagedState>,
    screen_id: u32,
    panel_id: u8,
) -> Result<(), CommandError> {
    let mut runtime = state.inner.runtime.write().await;
    runtime.set_active_panel(screen_id, panel_id)
}

#[tauri::command]
pub async fn update_panel_config(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
    panel_id: u8,
    patch: PanelConfigPatch,
) -> Result<(), CommandError> {
    let key = PanelKey {
        screen_id,
        panel_id,
    };
    let outcome = {
        let mut runtime = state.inner.runtime.write().await;
        runtime.update_panel_config(key, patch)?
    };

    if outcome.was_playing && outcome.tuple_changed {
        restart_panel(&app, state.inner().clone(), key).await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn set_panel_secret(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
    panel_id: u8,
    username: String,
    password: String,
) -> Result<(), CommandError> {
    let key = PanelKey {
        screen_id,
        panel_id,
    };
    let outcome = {
        let mut runtime = state.inner.runtime.write().await;
        runtime.set_panel_secret(key, username, password)?
    };

    if outcome.was_playing && outcome.presence_changed {
        restart_panel(&app, state.inner().clone(), key).await?;
    }

    Ok(())
}

#[tauri::command]
pub async fn auto_populate_cameras(
    app: AppHandle,
    state: State<'_, ManagedState>,
    tool: AutoPopulateTool,
) -> Result<(), CommandError> {
    validate_auto_populate_tool(&tool)?;

    if tool.base_url_template.trim().is_empty() {
        return Err(CommandError::config(
            "base_url_template is required for auto-population",
        ));
    }

    let camera_numbers = (tool.camera_num_start..=tool.camera_num_end).collect::<Vec<_>>();
    if camera_numbers.is_empty() {
        return Err(CommandError::config("camera range is empty"));
    }

    struct Assignment {
        camera_num: u32,
        sub_num: u32,
        parsed: ParsedRtsp,
    }

    let mut assignments = Vec::with_capacity(camera_numbers.len());
    for camera_num in camera_numbers {
        let sub_num = tool.sub_num_start;
        let resolved_url = resolve_auto_populated_url(&tool, camera_num, sub_num);
        let parsed = parse_rtsp_url(&resolved_url)?;
        assignments.push(Assignment {
            camera_num,
            sub_num,
            parsed,
        });
    }

    let playing_before = { state.inner.runtime.read().await.playing_keys() };
    for key in playing_before {
        let _ = stop_panel(&app, state.inner().clone(), key, false).await;
    }

    {
        let mut runtime = state.inner.runtime.write().await;
        runtime.set_auto_populate_tool_value(tool.clone());

        let needed_screens = assignments.len().div_ceil(4);
        while runtime.screen_count() < needed_screens {
            runtime.create_screen()?;
        }
        while runtime.screen_count() > needed_screens {
            let last_index = runtime.screen_count().saturating_sub(1) as u32;
            runtime.delete_screen(last_index)?;
        }

        let total_panels = runtime.screen_count() * 4;
        for index in 0..total_panels {
            let key = PanelKey {
                screen_id: (index / 4) as u32,
                panel_id: (index % 4) as u8,
            };
            let panel = runtime.get_panel_mut(key)?;

            if index < assignments.len() {
                let assignment = &assignments[index];
                panel.config.title = format!("Camera {}", assignment.camera_num);
                panel.config.host = assignment.parsed.host.clone();
                panel.config.port = assignment.parsed.port;
                panel.config.path = assignment.parsed.path.clone();
                panel.config.channel = None;
                panel.config.subtype = None;
                panel.config.camera_num = Some(assignment.camera_num);
                panel.config.sub_num = Some(assignment.sub_num);
                panel.secret = if assignment.parsed.username.trim().is_empty()
                    && assignment.parsed.password.trim().is_empty()
                {
                    None
                } else {
                    Some(PanelSecret {
                        username: assignment.parsed.username.clone(),
                        password: assignment.parsed.password.clone(),
                    })
                };
            } else {
                panel.config = rtsp_core::default_panel_config(key.screen_id, key.panel_id);
                panel.secret = None;
            }

            panel.status = PanelRuntimeStatus::default();
            panel.latest_frame = None;
            panel.is_recording = false;
        }

        runtime.active_screen = 0;
        if !runtime.active_panel_per_screen.is_empty() {
            runtime.active_panel_per_screen[0] = 0;
        }
    }

    emit_security_notice(
        &app,
        "E_CONFIG_INVALID",
        "auto-population completed and reset active screen to 0",
    )
    .await?;

    Ok(())
}

#[tauri::command]
pub async fn start_stream(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
    panel_id: u8,
) -> Result<(), CommandError> {
    start_panel(
        &app,
        state.inner().clone(),
        PanelKey {
            screen_id,
            panel_id,
        },
    )
    .await
}

#[tauri::command]
pub async fn stop_stream(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
    panel_id: u8,
) -> Result<(), CommandError> {
    stop_panel(
        &app,
        state.inner().clone(),
        PanelKey {
            screen_id,
            panel_id,
        },
        true,
    )
    .await
}

#[tauri::command]
pub async fn start_screen(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
) -> Result<(), CommandError> {
    let mut first_error: Option<CommandError> = None;
    for panel_id in 0..4 {
        if let Err(error) = start_panel(
            &app,
            state.inner().clone(),
            PanelKey {
                screen_id,
                panel_id,
            },
        )
        .await
        {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_screen(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
) -> Result<(), CommandError> {
    for panel_id in 0..4 {
        stop_panel(
            &app,
            state.inner().clone(),
            PanelKey {
                screen_id,
                panel_id,
            },
            true,
        )
        .await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn start_all_global(
    app: AppHandle,
    state: State<'_, ManagedState>,
) -> Result<(), CommandError> {
    let screen_count = { state.inner.runtime.read().await.screen_count() as u32 };

    let mut first_error: Option<CommandError> = None;
    for screen_id in 0..screen_count {
        for panel_id in 0..4 {
            if let Err(error) = start_panel(
                &app,
                state.inner().clone(),
                PanelKey {
                    screen_id,
                    panel_id,
                },
            )
            .await
            {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_all_global(
    app: AppHandle,
    state: State<'_, ManagedState>,
) -> Result<(), CommandError> {
    let screen_count = { state.inner.runtime.read().await.screen_count() as u32 };
    for screen_id in 0..screen_count {
        for panel_id in 0..4 {
            stop_panel(
                &app,
                state.inner().clone(),
                PanelKey {
                    screen_id,
                    panel_id,
                },
                true,
            )
            .await?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn save_config(
    state: State<'_, ManagedState>,
    path: Option<String>,
) -> Result<String, CommandError> {
    let selected_path = resolve_save_path(path, "rtsp_viewer_config.json")?;
    let config = {
        let runtime = state.inner.runtime.read().await;
        runtime.to_app_config()
    };
    let serialized =
        serde_json::to_vec_pretty(&config).map_err(|error| CommandError::io(error.to_string()))?;
    atomic_write(&selected_path, &serialized)?;
    Ok(selected_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn load_config(
    app: AppHandle,
    state: State<'_, ManagedState>,
    path: Option<String>,
) -> Result<String, CommandError> {
    let selected_path = resolve_open_path(path)?;
    let content = tokio::fs::read_to_string(&selected_path).await?;
    let parsed: AppConfig = serde_json::from_str(&content)
        .map_err(|error| CommandError::config(format!("invalid config json: {}", error)))?;

    let outcome = {
        let mut runtime = state.inner.runtime.write().await;
        runtime.merge_loaded_config(parsed)?
    };

    for key in &outcome.stop_keys {
        let _ = stop_panel(&app, state.inner().clone(), *key, false).await;
    }

    for key in &outcome.restart_keys {
        let _ = start_panel(&app, state.inner().clone(), *key).await;
    }

    let snapshot = {
        let runtime = state.inner.runtime.read().await;
        runtime.snapshot()
    };

    events::emit_config_loaded(
        &app,
        ConfigLoadedEvent {
            ipc_version: IPC_VERSION.to_string(),
            state: snapshot,
        },
    )?;

    Ok(selected_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn snapshot(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
    panel_id: u8,
    path: Option<String>,
) -> Result<String, CommandError> {
    let key = PanelKey {
        screen_id,
        panel_id,
    };
    let default_name = format!(
        "snapshot_s{}_p{}_{}.jpg",
        screen_id,
        panel_id,
        Local::now().format("%Y%m%d_%H%M%S")
    );
    let selected_path = resolve_save_path(path, &default_name)?;

    let frame = {
        let runtime = state.inner.runtime.read().await;
        runtime.latest_frame(key)?
    }
    .ok_or_else(|| CommandError::decode("no frame available for snapshot"))?;

    let bytes = STANDARD
        .decode(frame.data_base64.as_bytes())
        .map_err(|error| CommandError::decode(error.to_string()))?;

    let result = tokio::fs::write(&selected_path, bytes).await;

    match result {
        Ok(_) => {
            events::emit_snapshot_saved(
                &app,
                SnapshotSavedEvent {
                    ipc_version: IPC_VERSION.to_string(),
                    screen_id,
                    panel_id,
                    path: selected_path.to_string_lossy().to_string(),
                },
            )?;
            Ok(selected_path.to_string_lossy().to_string())
        }
        Err(error) => {
            let message = error.to_string();
            let _ = events::emit_snapshot_failed(
                &app,
                SnapshotFailedEvent {
                    ipc_version: IPC_VERSION.to_string(),
                    screen_id,
                    panel_id,
                    code: "E_IO".to_string(),
                    message: message.clone(),
                },
            );
            Err(CommandError::io(message))
        }
    }
}

#[tauri::command]
pub async fn toggle_recording(
    state: State<'_, ManagedState>,
    screen_id: u32,
    panel_id: u8,
    path: Option<String>,
) -> Result<Option<String>, CommandError> {
    let key = PanelKey { screen_id, panel_id };
    let is_recording = {
        let runtime = state.inner.runtime.read().await;
        runtime.get_panel(key)?.is_recording
    };

    if !is_recording {
        let mut runtime = state.inner.runtime.write().await;
        runtime.set_recording(key, true)?;
        return Ok(None);
    }

    let default_name = format!(
        "recording_s{}_p{}_{}.mp4",
        screen_id,
        panel_id,
        Local::now().format("%Y%m%d_%H%M%S")
    );
    let selected_path = resolve_save_path(path, &default_name)?;
    let placeholder = b"stub recording output";
    tokio::fs::write(&selected_path, placeholder).await?;

    {
        let mut runtime = state.inner.runtime.write().await;
        if runtime.panel_exists(key) {
            runtime.set_recording(key, false)?;
        }
    }

    Ok(Some(selected_path.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn toggle_fullscreen(
    app: AppHandle,
    state: State<'_, ManagedState>,
    enabled: bool,
) -> Result<(), CommandError> {
    {
        let mut runtime = state.inner.runtime.write().await;
        runtime.fullscreen = enabled;
    }

    if let Some(window) = app.get_window("main") {
        window
            .set_fullscreen(enabled)
            .map_err(|error| CommandError::io(error.to_string()))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn create_screen(state: State<'_, ManagedState>) -> Result<u32, CommandError> {
    let mut runtime = state.inner.runtime.write().await;
    runtime.create_screen()
}

#[tauri::command]
pub async fn delete_screen(
    app: AppHandle,
    state: State<'_, ManagedState>,
    screen_id: u32,
) -> Result<(), CommandError> {
    let playing_before = { state.inner.runtime.read().await.playing_keys() };

    for key in &playing_before {
        let _ = stop_panel(&app, state.inner().clone(), *key, false).await;
    }

    {
        let mut runtime = state.inner.runtime.write().await;
        runtime.delete_screen(screen_id)?;
    }

    let mut to_restart = Vec::new();
    for key in playing_before {
        if key.screen_id == screen_id {
            continue;
        }
        let remapped = if key.screen_id > screen_id {
            PanelKey {
                screen_id: key.screen_id - 1,
                panel_id: key.panel_id,
            }
        } else {
            key
        };
        to_restart.push(remapped);
    }

    for key in to_restart {
        let _ = start_panel(&app, state.inner().clone(), key).await;
    }

    let _ = emit_security_notice(
        &app,
        "E_CONFIG_INVALID",
        "screen ids were reindexed to remain dense",
    )
    .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::atomic_write;

    #[test]
    fn atomic_write_creates_and_replaces_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let path = temp_dir.path().join("config.json");

        atomic_write(&path, b"{\"schema_version\":2}").expect("first write should succeed");
        let first = std::fs::read_to_string(&path).expect("file should exist");
        assert_eq!(first, "{\"schema_version\":2}");

        atomic_write(&path, b"{\"schema_version\":3}").expect("second write should succeed");
        let second = std::fs::read_to_string(&path).expect("file should still exist");
        assert_eq!(second, "{\"schema_version\":3}");
    }
}
