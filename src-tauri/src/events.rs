use crate::errors::CommandError;
use rtsp_core::{
    ConfigLoadedEvent, PanelFrameEvent, PanelStatusEvent, SnapshotFailedEvent, SnapshotSavedEvent,
};
use tauri::{AppHandle, Manager};

pub fn emit_panel_status(app: &AppHandle, payload: PanelStatusEvent) -> Result<(), CommandError> {
    app.emit_all("panel_status", payload)
        .map_err(|error| CommandError::internal(error.to_string()))
}

pub fn emit_panel_frame(app: &AppHandle, payload: PanelFrameEvent) -> Result<(), CommandError> {
    app.emit_all("panel_frame", payload)
        .map_err(|error| CommandError::internal(error.to_string()))
}

pub fn emit_config_loaded(app: &AppHandle, payload: ConfigLoadedEvent) -> Result<(), CommandError> {
    app.emit_all("config_loaded", payload)
        .map_err(|error| CommandError::internal(error.to_string()))
}

pub fn emit_snapshot_saved(
    app: &AppHandle,
    payload: SnapshotSavedEvent,
) -> Result<(), CommandError> {
    app.emit_all("snapshot_saved", payload)
        .map_err(|error| CommandError::internal(error.to_string()))
}

pub fn emit_snapshot_failed(
    app: &AppHandle,
    payload: SnapshotFailedEvent,
) -> Result<(), CommandError> {
    app.emit_all("snapshot_failed", payload)
        .map_err(|error| CommandError::internal(error.to_string()))
}
