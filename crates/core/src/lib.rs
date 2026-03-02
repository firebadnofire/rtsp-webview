use serde::{Deserialize, Serialize};
use std::array;

pub const IPC_VERSION: &str = "1";
pub const SCHEMA_VERSION: u32 = 2;
pub const DEFAULT_SCREEN_COUNT: u32 = 0;
pub const MAX_SCREEN_COUNT: usize = 32;
pub const PANELS_PER_SCREEN: usize = 4;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

impl ValidationError {
    pub fn code(&self) -> &'static str {
        "E_CONFIG_INVALID"
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::InvalidConfig(message) => message.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoPopulateTool {
    pub base_url_template: String,
    pub username: String,
    pub password: String,
    pub ip: String,
    pub port: String,
    pub camera_num_start: u32,
    pub camera_num_end: u32,
    pub sub_num_start: u32,
    pub sub_num_end: u32,
}

impl Default for AutoPopulateTool {
    fn default() -> Self {
        Self {
            base_url_template:
                "rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum"
                    .to_string(),
            username: String::new(),
            password: String::new(),
            ip: String::new(),
            port: "554".to_string(),
            camera_num_start: 1,
            camera_num_end: 16,
            sub_num_start: 0,
            sub_num_end: 1,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoPopulateToolPatch {
    pub base_url_template: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ip: Option<String>,
    pub port: Option<String>,
    pub camera_num_start: Option<u32>,
    pub camera_num_end: Option<u32>,
    pub sub_num_start: Option<u32>,
    pub sub_num_end: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Tcp,
    Udp,
}

impl Default for Transport {
    fn default() -> Self {
        Self::Tcp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelState {
    Idle,
    Connecting,
    Playing,
    Retrying,
    Error,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub connection_timeout_ms: u32,
    pub stall_timeout_ms: u32,
    pub retry_base_ms: u32,
    pub retry_max_ms: u32,
    pub retry_jitter_ms: u32,
    pub max_failures: u32,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            connection_timeout_ms: 5_000,
            stall_timeout_ms: 5_000,
            retry_base_ms: 500,
            retry_max_ms: 10_000,
            retry_jitter_ms: 250,
            max_failures: 30,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancedConfigPatch {
    pub connection_timeout_ms: Option<u32>,
    pub stall_timeout_ms: Option<u32>,
    pub retry_base_ms: Option<u32>,
    pub retry_max_ms: Option<u32>,
    pub retry_jitter_ms: Option<u32>,
    pub max_failures: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelConfig {
    pub title: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub channel: Option<String>,
    pub subtype: Option<String>,
    #[serde(default)]
    pub camera_num: Option<u32>,
    #[serde(default)]
    pub sub_num: Option<u32>,
    pub transport: Transport,
    pub latency_ms: u32,
    pub secret_ref: SecretRef,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelConfigPatch {
    pub title: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub channel: Option<Option<String>>,
    pub subtype: Option<Option<String>>,
    pub camera_num: Option<Option<u32>>,
    pub sub_num: Option<Option<u32>>,
    pub transport: Option<Transport>,
    pub latency_ms: Option<u32>,
    pub advanced: Option<AdvancedConfigPatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenConfig {
    pub id: u32,
    pub panels: [PanelConfig; PANELS_PER_SCREEN],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiState {
    pub active_screen: u32,
    pub active_panel_per_screen: Vec<u8>,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub schema_version: u32,
    pub screens: Vec<ScreenConfig>,
    pub ui_state: UiState,
    #[serde(default)]
    pub auto_populate_tool: AutoPopulateTool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelRuntimeStatus {
    pub state: PanelState,
    pub message: String,
    pub code: Option<String>,
}

impl Default for PanelRuntimeStatus {
    fn default() -> Self {
        Self {
            state: PanelState::Idle,
            message: "Idle".to_string(),
            code: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelStateView {
    pub config: PanelConfig,
    pub status: PanelRuntimeStatus,
    pub secret_present: bool,
    pub is_recording: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenStateView {
    pub id: u32,
    pub panels: [PanelStateView; PANELS_PER_SCREEN],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStateResponse {
    pub ipc_version: String,
    pub schema_version: u32,
    pub active_screen: u32,
    pub active_panel_per_screen: Vec<u8>,
    pub fullscreen: bool,
    pub screens: Vec<ScreenStateView>,
    pub auto_populate_tool: AutoPopulateTool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelStatusEvent {
    pub ipc_version: String,
    pub screen_id: u32,
    pub panel_id: u8,
    pub state: PanelState,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelFrameEvent {
    pub ipc_version: String,
    pub screen_id: u32,
    pub panel_id: u8,
    pub mime: String,
    pub data_base64: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub pts_ms: Option<u64>,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigLoadedEvent {
    pub ipc_version: String,
    pub state: GetStateResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotSavedEvent {
    pub ipc_version: String,
    pub screen_id: u32,
    pub panel_id: u8,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotFailedEvent {
    pub ipc_version: String,
    pub screen_id: u32,
    pub panel_id: u8,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityNoticeEvent {
    pub ipc_version: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionTuple {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub transport: Transport,
    pub has_credentials: bool,
    pub camera_num: Option<u32>,
    pub sub_num: Option<u32>,
}

pub fn default_panel_config(screen_id: u32, panel_id: u8) -> PanelConfig {
    PanelConfig {
        title: format!("Screen {} Panel {}", screen_id + 1, panel_id + 1),
        host: String::new(),
        port: 554,
        path: String::new(),
        channel: None,
        subtype: None,
        camera_num: None,
        sub_num: None,
        transport: Transport::Tcp,
        latency_ms: 200,
        secret_ref: SecretRef {
            key: secret_key_for(screen_id, panel_id),
        },
        advanced: AdvancedConfig::default(),
    }
}

pub fn default_screen_config(id: u32) -> ScreenConfig {
    ScreenConfig {
        id,
        panels: array::from_fn(|idx| default_panel_config(id, idx as u8)),
    }
}

pub fn default_app_config(screen_count: u32) -> AppConfig {
    let normalized = screen_count.min(MAX_SCREEN_COUNT as u32);
    let screens = (0..normalized)
        .map(default_screen_config)
        .collect::<Vec<_>>();
    AppConfig {
        schema_version: SCHEMA_VERSION,
        ui_state: UiState {
            active_screen: 0,
            active_panel_per_screen: vec![0; screens.len()],
            fullscreen: false,
        },
        screens,
        auto_populate_tool: AutoPopulateTool::default(),
    }
}

pub fn validate_panel_config(panel: &PanelConfig) -> Result<(), ValidationError> {
    if panel.port == 0 {
        return Err(ValidationError::InvalidConfig(
            "port must be between 1 and 65535".to_string(),
        ));
    }
    if panel.latency_ms > 5_000 {
        return Err(ValidationError::InvalidConfig(
            "latency_ms must be between 0 and 5000".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_auto_populate_tool(tool: &AutoPopulateTool) -> Result<(), ValidationError> {
    if tool.camera_num_start > tool.camera_num_end {
        return Err(ValidationError::InvalidConfig(
            "camera_num_start must be <= camera_num_end".to_string(),
        ));
    }
    if tool.sub_num_start > tool.sub_num_end {
        return Err(ValidationError::InvalidConfig(
            "sub_num_start must be <= sub_num_end".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_app_config(config: &AppConfig) -> Result<(), ValidationError> {
    if config.schema_version != SCHEMA_VERSION {
        return Err(ValidationError::InvalidConfig(format!(
            "unsupported schema_version {}",
            config.schema_version
        )));
    }

    if config.screens.len() > MAX_SCREEN_COUNT {
        return Err(ValidationError::InvalidConfig(
            "screens must be between 0 and 32".to_string(),
        ));
    }

    for (idx, screen) in config.screens.iter().enumerate() {
        if screen.id != idx as u32 {
            return Err(ValidationError::InvalidConfig(
                "screen ids must be dense and start at 0".to_string(),
            ));
        }
        for panel in &screen.panels {
            validate_panel_config(panel)?;
        }
    }

    if config.screens.is_empty() {
        if config.ui_state.active_screen != 0 {
            return Err(ValidationError::InvalidConfig(
                "active_screen must be 0 when no screens exist".to_string(),
            ));
        }
        if !config.ui_state.active_panel_per_screen.is_empty() {
            return Err(ValidationError::InvalidConfig(
                "active_panel_per_screen must be empty when no screens exist".to_string(),
            ));
        }
    } else {
        let max_screen = (config.screens.len() - 1) as u32;
        if config.ui_state.active_screen > max_screen {
            return Err(ValidationError::InvalidConfig(
                "active_screen must exist in screens".to_string(),
            ));
        }

        if config.ui_state.active_panel_per_screen.len() != config.screens.len() {
            return Err(ValidationError::InvalidConfig(
                "active_panel_per_screen length must match screens length".to_string(),
            ));
        }

        for panel in &config.ui_state.active_panel_per_screen {
            if *panel > 3 {
                return Err(ValidationError::InvalidConfig(
                    "active_panel_per_screen values must be in 0..=3".to_string(),
                ));
            }
        }
    }

    validate_auto_populate_tool(&config.auto_populate_tool)
}

pub fn apply_panel_patch(
    panel: &mut PanelConfig,
    patch: PanelConfigPatch,
) -> Result<(), ValidationError> {
    if let Some(title) = patch.title {
        panel.title = title;
    }
    if let Some(host) = patch.host {
        panel.host = host;
    }
    if let Some(port) = patch.port {
        panel.port = port;
    }
    if let Some(path) = patch.path {
        panel.path = path;
    }
    if let Some(channel) = patch.channel {
        panel.channel = channel;
    }
    if let Some(subtype) = patch.subtype {
        panel.subtype = subtype;
    }
    if let Some(camera_num) = patch.camera_num {
        panel.camera_num = camera_num;
    }
    if let Some(sub_num) = patch.sub_num {
        panel.sub_num = sub_num;
    }
    if let Some(transport) = patch.transport {
        panel.transport = transport;
    }
    if let Some(latency_ms) = patch.latency_ms {
        panel.latency_ms = latency_ms;
    }
    if let Some(advanced_patch) = patch.advanced {
        apply_advanced_patch(&mut panel.advanced, advanced_patch);
    }
    validate_panel_config(panel)
}

pub fn apply_auto_populate_tool_patch(
    tool: &mut AutoPopulateTool,
    patch: AutoPopulateToolPatch,
) -> Result<(), ValidationError> {
    if let Some(value) = patch.base_url_template {
        tool.base_url_template = value;
    }
    if let Some(value) = patch.username {
        tool.username = value;
    }
    if let Some(value) = patch.password {
        tool.password = value;
    }
    if let Some(value) = patch.ip {
        tool.ip = value;
    }
    if let Some(value) = patch.port {
        tool.port = value;
    }
    if let Some(value) = patch.camera_num_start {
        tool.camera_num_start = value;
    }
    if let Some(value) = patch.camera_num_end {
        tool.camera_num_end = value;
    }
    if let Some(value) = patch.sub_num_start {
        tool.sub_num_start = value;
    }
    if let Some(value) = patch.sub_num_end {
        tool.sub_num_end = value;
    }
    validate_auto_populate_tool(tool)
}

fn apply_advanced_patch(advanced: &mut AdvancedConfig, patch: AdvancedConfigPatch) {
    if let Some(value) = patch.connection_timeout_ms {
        advanced.connection_timeout_ms = value;
    }
    if let Some(value) = patch.stall_timeout_ms {
        advanced.stall_timeout_ms = value;
    }
    if let Some(value) = patch.retry_base_ms {
        advanced.retry_base_ms = value;
    }
    if let Some(value) = patch.retry_max_ms {
        advanced.retry_max_ms = value;
    }
    if let Some(value) = patch.retry_jitter_ms {
        advanced.retry_jitter_ms = value;
    }
    if let Some(value) = patch.max_failures {
        advanced.max_failures = value;
    }
}

pub fn secret_key_for(screen_id: u32, panel_id: u8) -> String {
    format!("screen_{}_panel_{}", screen_id, panel_id)
}

pub fn connection_tuple(panel: &PanelConfig, has_credentials: bool) -> ConnectionTuple {
    ConnectionTuple {
        host: panel.host.clone(),
        port: panel.port,
        path: panel.path.clone(),
        transport: panel.transport,
        has_credentials,
        camera_num: panel.camera_num,
        sub_num: panel.sub_num,
    }
}

pub fn build_rtsp_url(
    panel: &PanelConfig,
    username: Option<&str>,
    password: Option<&str>,
    reveal_password: bool,
) -> String {
    let mut credential = String::new();
    if let Some(user) = username {
        if !user.is_empty() {
            credential.push_str(user);
            if let Some(pass) = password {
                if !pass.is_empty() {
                    credential.push(':');
                    if reveal_password {
                        credential.push_str(pass);
                    } else {
                        credential.push_str("***");
                    }
                }
            }
            credential.push('@');
        }
    }

    let mut path = panel.path.trim_start_matches('/').to_string();
    if let Some(channel) = &panel.channel {
        if !channel.is_empty() {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(channel);
        }
    }
    if let Some(subtype) = &panel.subtype {
        if !subtype.is_empty() {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(subtype);
        }
    }

    format!(
        "rtsp://{}{}:{}/{}",
        credential, panel.host, panel.port, path
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_zero_screen_config_is_valid() {
        let config = default_app_config(DEFAULT_SCREEN_COUNT);
        assert!(validate_app_config(&config).is_ok());
        assert_eq!(config.screens.len(), 0);
    }

    #[test]
    fn invalid_latency_is_rejected() {
        let mut config = default_app_config(1);
        config.screens[0].panels[0].latency_ms = 9_000;
        let error = validate_app_config(&config).expect_err("expected validation error");
        assert_eq!(error.code(), "E_CONFIG_INVALID");
    }

    #[test]
    fn invalid_tool_ranges_are_rejected() {
        let mut config = default_app_config(1);
        config.auto_populate_tool.camera_num_start = 9;
        config.auto_populate_tool.camera_num_end = 1;
        assert!(validate_app_config(&config).is_err());
    }

    #[test]
    fn rtsp_preview_masks_password_by_default() {
        let mut panel = default_panel_config(0, 0);
        panel.host = "127.0.0.1".to_string();
        panel.path = "stream".to_string();
        let masked = build_rtsp_url(&panel, Some("user"), Some("secret"), false);
        assert!(masked.contains("user:***@"));
        assert!(!masked.contains("secret"));

        let revealed = build_rtsp_url(&panel, Some("user"), Some("secret"), true);
        assert!(revealed.contains("user:secret@"));
    }
}
