use crate::app_state::ManagedState;
use crate::errors::CommandError;
use crate::state::{PanelKey, PanelSecret};
use crate::{events, stub_streams};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Local;
use percent_encoding::percent_decode_str;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rtsp_core::{
    validate_auto_populate_tool, AppConfig, AutoPopulateTool, ConfigLoadedEvent, GetStateResponse,
    PanelConfigPatch, PanelRuntimeStatus, PanelState, PanelStatusEvent, SnapshotFailedEvent,
    SnapshotSavedEvent, IPC_VERSION, MAX_SCREEN_COUNT, PANELS_PER_SCREEN,
};
use rtsp_secrets::SecretPayload;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, collections::HashSet};
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

fn encode_userinfo_value(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn resolve_auto_populated_url(tool: &AutoPopulateTool, camera_num: u32, sub_num: u32) -> String {
    let mut output = tool.base_url_template.clone();
    output = replace_token(&output, "$cameraNum", &camera_num.to_string());
    output = replace_token(&output, "$subNum", &sub_num.to_string());
    output = replace_token(&output, "$USERNAME", &encode_userinfo_value(&tool.username));
    output = replace_token(&output, "$PASSWORD", &encode_userinfo_value(&tool.password));
    output = replace_token(&output, "$IP", &tool.ip);
    output = replace_token(&output, "$PORT", &tool.port);
    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Assignment {
    camera_num: u32,
    sub_num: u32,
    parsed: ParsedRtsp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    let username = percent_decode_str(parsed.username())
        .decode_utf8()
        .map_err(|error| CommandError::config(format!("invalid username encoding: {}", error)))?
        .into_owned();
    let password = percent_decode_str(parsed.password().unwrap_or_default())
        .decode_utf8()
        .map_err(|error| CommandError::config(format!("invalid password encoding: {}", error)))?
        .into_owned();

    Ok(ParsedRtsp {
        host,
        port,
        path,
        username,
        password,
    })
}

fn build_auto_populate_assignments(
    tool: &AutoPopulateTool,
) -> Result<Vec<Assignment>, CommandError> {
    let camera_numbers = (tool.camera_num_start..=tool.camera_num_end).collect::<Vec<_>>();
    let subtype_numbers = (tool.sub_num_start..=tool.sub_num_end).collect::<Vec<_>>();
    if camera_numbers.is_empty() {
        return Err(CommandError::config("camera range is empty"));
    }
    if subtype_numbers.is_empty() {
        return Err(CommandError::config("subtype range is empty"));
    }

    let total_assignments = camera_numbers.len();
    let max_panels = MAX_SCREEN_COUNT * PANELS_PER_SCREEN;
    if total_assignments > max_panels {
        return Err(CommandError::config(format!(
            "auto-population would generate {} panels, exceeding max {}",
            total_assignments, max_panels
        )));
    }

    let mut assignments = Vec::with_capacity(total_assignments);
    let default_sub_num = subtype_numbers[0];
    for camera_num in camera_numbers {
        let resolved_url = resolve_auto_populated_url(tool, camera_num, default_sub_num);
        let parsed = parse_rtsp_url(&resolved_url)?;
        assignments.push(Assignment {
            camera_num,
            sub_num: default_sub_num,
            parsed,
        });
    }
    Ok(assignments)
}

fn apply_secret_updates(
    state: &ManagedState,
    desired_secrets: HashMap<String, Option<PanelSecret>>,
    existing_secret_keys: HashSet<String>,
) -> Result<(), CommandError> {
    let mut touched_keys = HashSet::new();

    for (key, value) in desired_secrets {
        touched_keys.insert(key.clone());
        if let Some(secret) = value {
            state.inner.secret_store.set(
                &key,
                SecretPayload {
                    username: secret.username,
                    password: secret.password,
                },
            )?;
        } else {
            state.inner.secret_store.delete(&key)?;
        }
    }

    for stale_key in existing_secret_keys {
        if !touched_keys.contains(&stale_key) {
            state.inner.secret_store.delete(&stale_key)?;
        }
    }

    Ok(())
}

fn load_secrets_for_config(
    state: &ManagedState,
    config: &AppConfig,
) -> Result<HashMap<String, PanelSecret>, CommandError> {
    let mut map = HashMap::new();
    for (screen_idx, _) in config.screens.iter().enumerate() {
        for panel_idx in 0..PANELS_PER_SCREEN {
            let key = rtsp_core::secret_key_for(screen_idx as u32, panel_idx as u8);
            if map.contains_key(&key) {
                continue;
            }
            if let Some(payload) = state.inner.secret_store.get(&key)? {
                if payload.username.trim().is_empty() && payload.password.trim().is_empty() {
                    continue;
                }
                map.insert(
                    key,
                    PanelSecret {
                        username: payload.username,
                        password: payload.password,
                    },
                );
            }
        }
    }
    Ok(map)
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
    let secret_key = {
        let runtime = state.inner.runtime.read().await;
        runtime.get_panel(key)?.config.secret_ref.key.clone()
    };

    if username.trim().is_empty() && password.trim().is_empty() {
        state.inner.secret_store.delete(&secret_key)?;
    } else {
        state.inner.secret_store.set(
            &secret_key,
            SecretPayload {
                username: username.clone(),
                password: password.clone(),
            },
        )?;
    }

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

    let assignments = build_auto_populate_assignments(&tool)?;
    let existing_secret_keys = {
        let runtime = state.inner.runtime.read().await;
        let mut keys = HashSet::new();
        for screen in &runtime.screens {
            for panel in &screen.panels {
                if panel.secret.as_ref().is_some_and(PanelSecret::is_present) {
                    keys.insert(panel.config.secret_ref.key.clone());
                }
            }
        }
        keys
    };

    let playing_before = { state.inner.runtime.read().await.playing_keys() };
    for key in playing_before {
        let _ = stop_panel(&app, state.inner().clone(), key, false).await;
    }

    let mut desired_secrets = HashMap::new();
    {
        let mut runtime = state.inner.runtime.write().await;
        runtime.set_auto_populate_tool_value(tool.clone());

        let needed_screens = assignments.len().div_ceil(PANELS_PER_SCREEN);
        while runtime.screen_count() < needed_screens {
            runtime.create_screen()?;
        }
        while runtime.screen_count() > needed_screens {
            let last_index = runtime.screen_count().saturating_sub(1) as u32;
            runtime.delete_screen(last_index)?;
        }

        let total_panels = runtime.screen_count() * PANELS_PER_SCREEN;
        for index in 0..total_panels {
            let key = PanelKey {
                screen_id: (index / PANELS_PER_SCREEN) as u32,
                panel_id: (index % PANELS_PER_SCREEN) as u8,
            };
            let panel = runtime.get_panel_mut(key)?;
            let mut panel_secret = None;

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
                panel_secret = if assignment.parsed.username.trim().is_empty()
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
            }

            let secret_key = panel.config.secret_ref.key.clone();
            desired_secrets.insert(secret_key, panel_secret.clone());
            panel.secret = panel_secret;
            panel.status = PanelRuntimeStatus::default();
            panel.latest_frame = None;
            panel.is_recording = false;
        }

        runtime.active_screen = 0;
        for active_panel in &mut runtime.active_panel_per_screen {
            *active_panel = 0;
        }
    }

    apply_secret_updates(state.inner(), desired_secrets, existing_secret_keys)?;

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
    for panel_id in 0..PANELS_PER_SCREEN as u8 {
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
    for panel_id in 0..PANELS_PER_SCREEN as u8 {
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
        for panel_id in 0..PANELS_PER_SCREEN as u8 {
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
        for panel_id in 0..PANELS_PER_SCREEN as u8 {
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
    let external_secrets = load_secrets_for_config(state.inner(), &parsed)?;

    let outcome = {
        let mut runtime = state.inner.runtime.write().await;
        runtime.merge_loaded_config(parsed, external_secrets)?
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
    let key = PanelKey {
        screen_id,
        panel_id,
    };
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        apply_secret_updates, atomic_write, build_auto_populate_assignments,
        resolve_auto_populated_url,
    };
    use crate::app_state::ManagedState;
    use crate::state::PanelSecret;
    use rtsp_core::{AutoPopulateTool, PANELS_PER_SCREEN};
    use rtsp_secrets::{SecretError, SecretPayload, SecretStore};
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockSecretStore {
        values: Mutex<HashMap<String, SecretPayload>>,
        set_keys: Mutex<Vec<String>>,
        delete_keys: Mutex<Vec<String>>,
    }

    impl SecretStore for MockSecretStore {
        fn set(&self, key: &str, payload: SecretPayload) -> Result<(), SecretError> {
            self.values
                .lock()
                .expect("lock should succeed")
                .insert(key.to_string(), payload);
            self.set_keys
                .lock()
                .expect("lock should succeed")
                .push(key.to_string());
            Ok(())
        }

        fn get(&self, key: &str) -> Result<Option<SecretPayload>, SecretError> {
            Ok(self
                .values
                .lock()
                .expect("lock should succeed")
                .get(key)
                .cloned())
        }

        fn delete(&self, key: &str) -> Result<(), SecretError> {
            self.values.lock().expect("lock should succeed").remove(key);
            self.delete_keys
                .lock()
                .expect("lock should succeed")
                .push(key.to_string());
            Ok(())
        }
    }

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

    fn sample_tool() -> AutoPopulateTool {
        AutoPopulateTool {
            base_url_template:
                "rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum"
                    .to_string(),
            username: "admin".to_string(),
            password: "p@ss:word".to_string(),
            ip: "127.0.0.1".to_string(),
            port: "5554".to_string(),
            camera_num_start: 1,
            camera_num_end: 2,
            sub_num_start: 0,
            sub_num_end: 1,
        }
    }

    #[test]
    fn auto_populate_url_encodes_username_and_password_only() {
        let tool = sample_tool();
        let resolved = resolve_auto_populated_url(&tool, 3, 1);
        assert!(resolved.contains("admin:p%40ss%3Aword@"));
        assert!(resolved.contains("channel=3&subtype=1"));
        assert!(!resolved.contains("p@ss:word@"));
    }

    #[test]
    fn assignment_generation_uses_one_panel_per_camera_with_default_subtype() {
        let assignments = build_auto_populate_assignments(&sample_tool())
            .expect("assignment generation should succeed");
        let ordered_pairs = assignments
            .iter()
            .map(|assignment| (assignment.camera_num, assignment.sub_num))
            .collect::<Vec<_>>();
        assert_eq!(ordered_pairs, vec![(1, 0), (2, 0)]);
    }

    #[test]
    fn assignment_generation_computes_expected_screen_packing() {
        let mut tool = sample_tool();
        tool.camera_num_end = 5;
        tool.sub_num_end = 1;
        let assignments =
            build_auto_populate_assignments(&tool).expect("assignment generation should succeed");
        let needed_screens = assignments.len().div_ceil(PANELS_PER_SCREEN);
        assert_eq!(assignments.len(), 5);
        assert_eq!(needed_screens, 2);
    }

    #[test]
    fn assignment_generation_rejects_over_capacity_ranges() {
        let mut tool = sample_tool();
        tool.camera_num_end = 400;
        tool.sub_num_end = 1;
        let error =
            build_auto_populate_assignments(&tool).expect_err("range should exceed capacity");
        assert_eq!(error.code, "E_CONFIG_INVALID");
        assert!(error.message.contains("exceeding max"));
    }

    #[test]
    fn secret_update_applies_set_delete_and_stale_cleanup() {
        let store = Arc::new(MockSecretStore::default());
        let managed = ManagedState::with_secret_store(store.clone());

        let mut desired = HashMap::new();
        desired.insert(
            "screen_0_panel_0".to_string(),
            Some(PanelSecret {
                username: "user".to_string(),
                password: "secret".to_string(),
            }),
        );
        desired.insert("screen_0_panel_1".to_string(), None);

        let existing = HashSet::from([
            "screen_0_panel_0".to_string(),
            "screen_0_panel_1".to_string(),
            "screen_2_panel_3".to_string(),
        ]);

        apply_secret_updates(&managed, desired, existing).expect("secret updates should succeed");

        let set_keys = store.set_keys.lock().expect("lock should succeed").clone();
        let delete_keys = store
            .delete_keys
            .lock()
            .expect("lock should succeed")
            .clone();

        assert_eq!(set_keys, vec!["screen_0_panel_0".to_string()]);
        assert!(delete_keys.contains(&"screen_0_panel_1".to_string()));
        assert!(delete_keys.contains(&"screen_2_panel_3".to_string()));
    }
}
