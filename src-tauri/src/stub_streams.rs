use crate::app_state::{ManagedState, StreamTask};
use crate::errors::CommandError;
use crate::events;
use crate::state::{FrameCache, PanelKey};
use rtsp_core::{PanelFrameEvent, PanelState, PanelStatusEvent, IPC_VERSION};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tokio::time::{interval, sleep, timeout};
use tokio_util::sync::CancellationToken;

const STUB_JPEG_BASE64: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wCEAAkGBxISEhUTEhIVFRUVFRUVFRUVFRUVFRUXFRUWFhUVFRUYHSggGBolGxUVITEhJSkrLi4uFx8zODMtNygtLisBCgoKDg0OFQ8QFS0dFR0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLS0tLf/AABEIAAEAAQMBIgACEQEDEQH/xAAXAAADAQAAAAAAAAAAAAAAAAAAAQID/8QAFhEBAQEAAAAAAAAAAAAAAAAAABEB/9oADAMBAAIQAxAAAAGmAP/EABkQAAMAAwAAAAAAAAAAAAAAAAABAhEhMf/aAAgBAQABBQJrKrqf/8QAFhEAAwAAAAAAAAAAAAAAAAAAARAR/9oACAEDAQE/AUf/xAAVEQEBAAAAAAAAAAAAAAAAAAABEP/aAAgBAgEBPwFH/8QAGhAAAwADAQAAAAAAAAAAAAAAAQIRAyExQf/aAAgBAQAGPwK5FMx8W//EABsQAQACAwEBAAAAAAAAAAAAAAERIQAxQVFh/9oACAEBAAE/IdJLaQf0aR6Q3WqK2t8Yw7f/2gAMAwEAAgADAAAAEO//xAAXEQEBAQEAAAAAAAAAAAAAAAABABEh/9oACAEDAQE/EFj/xAAXEQEBAQEAAAAAAAAAAAAAAAABABEh/9oACAECAQE/EFj/xAAbEAEBAQADAQEAAAAAAAAAAAABEQAhMUFhcf/aAAgBAQABPxB4m2a6E0E8XnQ6Z7fXQ+PThIxj6NpjTrf/2Q==";

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
    sleep(Duration::from_millis(250)).await;
    if cancel.is_cancelled() {
        return Ok(());
    }

    {
        let mut runtime = managed.inner.runtime.write().await;
        if !runtime.panel_exists(key) {
            return Ok(());
        }
        runtime.set_panel_status(key, PanelState::Playing, "Playing", None)?;
    }

    events::emit_panel_status(
        &app,
        PanelStatusEvent {
            ipc_version: IPC_VERSION.to_string(),
            screen_id: key.screen_id,
            panel_id: key.panel_id,
            state: PanelState::Playing,
            message: "Playing".to_string(),
            code: None,
        },
    )?;

    let mut frame_seq: u64 = 0;
    let mut ticker = interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            _ = ticker.tick() => {
                frame_seq = frame_seq.saturating_add(1);
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                {
                    let mut runtime = managed.inner.runtime.write().await;
                    if !runtime.panel_exists(key) {
                        break;
                    }
                    runtime.set_latest_frame(
                        key,
                        FrameCache {
                            data_base64: STUB_JPEG_BASE64.to_string(),
                        },
                    )?;
                }

                let _ = events::emit_panel_frame(
                    &app,
                    PanelFrameEvent {
                        ipc_version: IPC_VERSION.to_string(),
                        screen_id: key.screen_id,
                        panel_id: key.panel_id,
                        mime: "image/jpeg".to_string(),
                        data_base64: STUB_JPEG_BASE64.to_string(),
                        width: Some(1),
                        height: Some(1),
                        pts_ms: Some(now_ms),
                        seq: frame_seq,
                    },
                );
            }
        }
    }

    Ok(())
}
