use crate::errors::CommandError;
use rtsp_core::{
    apply_panel_patch, apply_stream_defaults_patch, connection_tuple, default_app_config,
    default_screen_config, managed_preview_fps, secret_key_for, validate_app_config, AppConfig,
    AutoPopulateTool, ConnectionTuple, GetStateResponse, PanelConfig, PanelConfigPatch,
    PanelRuntimeStatus, PanelState, PanelStateView, ScreenStateView, StreamDefaults,
    StreamDefaultsPatch, DEFAULT_SCREEN_COUNT, SCHEMA_VERSION,
};
use std::array;
use std::collections::HashMap;

const AUTO_PREVIEW_FPS_PRIORITY_WEIGHT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelKey {
    pub screen_id: u32,
    pub panel_id: u8,
}

#[derive(Debug, Clone)]
pub struct PanelSecret {
    pub username: String,
    pub password: String,
}

impl PanelSecret {
    pub fn is_present(&self) -> bool {
        !self.username.trim().is_empty() || !self.password.trim().is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct FrameCache {
    pub mime: String,
    pub data_base64: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub pts_ms: Option<u64>,
    pub seq: u64,
}

#[derive(Debug, Clone)]
pub struct PanelRuntime {
    pub config: PanelConfig,
    pub status: PanelRuntimeStatus,
    pub secret: Option<PanelSecret>,
    pub preview_fps_in_use: Option<u8>,
    pub latest_frame: Option<FrameCache>,
    pub is_recording: bool,
}

#[derive(Debug, Clone)]
pub struct ScreenRuntime {
    pub id: u32,
    pub panels: [PanelRuntime; 4],
}

#[derive(Debug, Clone)]
pub struct AppRuntimeState {
    pub schema_version: u32,
    pub screens: Vec<ScreenRuntime>,
    pub active_screen: u32,
    pub active_panel_per_screen: Vec<u8>,
    pub fullscreen: bool,
    pub auto_populate_tool: AutoPopulateTool,
    pub stream_defaults: StreamDefaults,
}

#[derive(Debug, Clone)]
pub struct UpdatePanelOutcome {
    pub was_playing: bool,
    pub restart_required: bool,
}

#[derive(Debug, Clone)]
pub struct SetSecretOutcome {
    pub was_playing: bool,
    pub presence_changed: bool,
}

#[derive(Debug, Clone)]
pub struct LoadMergeOutcome {
    pub stop_keys: Vec<PanelKey>,
    pub restart_keys: Vec<PanelKey>,
}

#[derive(Debug, Clone)]
pub struct UpdateStreamDefaultsOutcome {
    pub restart_keys: Vec<PanelKey>,
}

fn is_active_stream_state(state: PanelState) -> bool {
    matches!(
        state,
        PanelState::Connecting | PanelState::Playing | PanelState::Retrying
    )
}

fn managed_preview_fps_with_priority(
    defaults: &StreamDefaults,
    active_stream_count: usize,
    prioritize_panel: bool,
    has_priority_panel: bool,
) -> u8 {
    if !defaults.auto_manage_preview_fps {
        return defaults.preview_fps;
    }

    if !has_priority_panel {
        return managed_preview_fps(defaults, active_stream_count);
    }

    let active_stream_count = active_stream_count.max(1);
    let total_budget =
        u16::from(defaults.preview_fps) * rtsp_core::AUTO_PREVIEW_FPS_REFERENCE_STREAMS as u16;
    let share_weight = if prioritize_panel {
        AUTO_PREVIEW_FPS_PRIORITY_WEIGHT as u16
    } else {
        1
    };
    let total_weight =
        (active_stream_count.saturating_sub(1) + AUTO_PREVIEW_FPS_PRIORITY_WEIGHT) as u16;
    let scaled = (total_budget * share_weight) / total_weight.max(1);
    scaled.clamp(
        u16::from(rtsp_core::MIN_PREVIEW_FPS),
        u16::from(defaults.preview_fps),
    ) as u8
}

impl AppRuntimeState {
    pub fn new_default() -> Self {
        let config = default_app_config(DEFAULT_SCREEN_COUNT);
        Self::from_config(config, HashMap::new())
    }

    pub fn from_config(config: AppConfig, mut secrets: HashMap<String, PanelSecret>) -> Self {
        let mut screens = Vec::with_capacity(config.screens.len());
        for (screen_idx, screen_cfg) in config.screens.iter().enumerate() {
            let panels = array::from_fn(|panel_idx| {
                let mut cfg = screen_cfg.panels[panel_idx].clone();
                cfg.secret_ref.key = secret_key_for(screen_idx as u32, panel_idx as u8);
                PanelRuntime {
                    config: cfg.clone(),
                    status: PanelRuntimeStatus::default(),
                    secret: secrets.remove(&cfg.secret_ref.key),
                    preview_fps_in_use: None,
                    latest_frame: None,
                    is_recording: false,
                }
            });
            screens.push(ScreenRuntime {
                id: screen_idx as u32,
                panels,
            });
        }

        Self {
            schema_version: SCHEMA_VERSION,
            active_screen: config.ui_state.active_screen,
            active_panel_per_screen: config.ui_state.active_panel_per_screen,
            fullscreen: config.ui_state.fullscreen,
            screens,
            auto_populate_tool: config.auto_populate_tool,
            stream_defaults: config.stream_defaults,
        }
    }

    pub fn screen_count(&self) -> usize {
        self.screens.len()
    }

    pub fn snapshot(&self) -> GetStateResponse {
        let screens = self
            .screens
            .iter()
            .map(|screen| ScreenStateView {
                id: screen.id,
                panels: array::from_fn(|panel_idx| {
                    let panel = &screen.panels[panel_idx];
                    PanelStateView {
                        config: panel.config.clone(),
                        status: panel.status.clone(),
                        secret_present: panel.secret.as_ref().is_some_and(PanelSecret::is_present),
                        is_recording: panel.is_recording,
                    }
                }),
            })
            .collect::<Vec<_>>();

        GetStateResponse {
            ipc_version: rtsp_core::IPC_VERSION.to_string(),
            schema_version: self.schema_version,
            active_screen: self.active_screen,
            active_panel_per_screen: self.active_panel_per_screen.clone(),
            fullscreen: self.fullscreen,
            screens,
            auto_populate_tool: self.auto_populate_tool.clone(),
            stream_defaults: self.stream_defaults.clone(),
        }
    }

    pub fn to_app_config(&self) -> AppConfig {
        AppConfig {
            schema_version: self.schema_version,
            screens: self
                .screens
                .iter()
                .map(|screen| rtsp_core::ScreenConfig {
                    id: screen.id,
                    panels: array::from_fn(|panel_idx| screen.panels[panel_idx].config.clone()),
                })
                .collect::<Vec<_>>(),
            ui_state: rtsp_core::UiState {
                active_screen: self.active_screen,
                active_panel_per_screen: self.active_panel_per_screen.clone(),
                fullscreen: self.fullscreen,
            },
            auto_populate_tool: self.auto_populate_tool.clone(),
            stream_defaults: self.stream_defaults.clone(),
            saved_secrets: HashMap::new(),
        }
    }

    fn validate_key(&self, key: PanelKey) -> Result<(), CommandError> {
        if key.panel_id > 3 {
            return Err(CommandError::config("panel_id must be in 0..=3"));
        }
        if key.screen_id as usize >= self.screens.len() {
            return Err(CommandError::config("screen_id does not exist"));
        }
        Ok(())
    }

    pub fn set_active_screen(&mut self, screen_id: u32) -> Result<(), CommandError> {
        if self.screens.is_empty() {
            return Err(CommandError::config("no screens available"));
        }
        if screen_id as usize >= self.screens.len() {
            return Err(CommandError::config("screen_id does not exist"));
        }
        self.active_screen = screen_id;
        Ok(())
    }

    pub fn set_active_panel(&mut self, screen_id: u32, panel_id: u8) -> Result<(), CommandError> {
        if panel_id > 3 {
            return Err(CommandError::config("panel_id must be in 0..=3"));
        }
        if screen_id as usize >= self.screens.len() {
            return Err(CommandError::config("screen_id does not exist"));
        }
        self.active_panel_per_screen[screen_id as usize] = panel_id;
        Ok(())
    }

    pub fn get_panel(&self, key: PanelKey) -> Result<&PanelRuntime, CommandError> {
        self.validate_key(key)?;
        Ok(&self.screens[key.screen_id as usize].panels[key.panel_id as usize])
    }

    pub fn get_panel_mut(&mut self, key: PanelKey) -> Result<&mut PanelRuntime, CommandError> {
        self.validate_key(key)?;
        Ok(&mut self.screens[key.screen_id as usize].panels[key.panel_id as usize])
    }

    pub fn panel_exists(&self, key: PanelKey) -> bool {
        key.panel_id <= 3 && key.screen_id as usize <= self.screens.len().saturating_sub(1)
    }

    pub fn update_panel_config(
        &mut self,
        key: PanelKey,
        patch: PanelConfigPatch,
    ) -> Result<UpdatePanelOutcome, CommandError> {
        let active_stream_count = self.active_stream_count();
        let old_tuple = self.panel_connection_tuple(key)?;
        let old_preview_fps =
            self.desired_preview_fps_for_key_with_active_count(key, active_stream_count)?;
        let was_playing = is_active_stream_state(self.get_panel(key)?.status.state);
        {
            let panel = self.get_panel_mut(key)?;
            apply_panel_patch(&mut panel.config, patch)?;
        }
        let new_tuple = self.panel_connection_tuple(key)?;
        let new_preview_fps =
            self.desired_preview_fps_for_key_with_active_count(key, active_stream_count)?;
        Ok(UpdatePanelOutcome {
            was_playing,
            restart_required: old_tuple != new_tuple || old_preview_fps != new_preview_fps,
        })
    }

    pub fn set_auto_populate_tool_value(&mut self, value: AutoPopulateTool) {
        self.auto_populate_tool = value;
    }

    pub fn update_stream_defaults(
        &mut self,
        patch: StreamDefaultsPatch,
    ) -> Result<UpdateStreamDefaultsOutcome, CommandError> {
        apply_stream_defaults_patch(&mut self.stream_defaults, patch)?;
        Ok(UpdateStreamDefaultsOutcome {
            restart_keys: self.preview_fps_rebalance_keys(None)?,
        })
    }

    pub fn set_panel_secret(
        &mut self,
        key: PanelKey,
        username: String,
        password: String,
    ) -> Result<SetSecretOutcome, CommandError> {
        let panel = self.get_panel_mut(key)?;
        let old_present = panel.secret.as_ref().is_some_and(PanelSecret::is_present);

        if username.trim().is_empty() && password.trim().is_empty() {
            panel.secret = None;
        } else {
            panel.secret = Some(PanelSecret { username, password });
        }

        let new_present = panel.secret.as_ref().is_some_and(PanelSecret::is_present);
        let was_playing = is_active_stream_state(panel.status.state);

        Ok(SetSecretOutcome {
            was_playing,
            presence_changed: old_present != new_present,
        })
    }

    pub fn set_panel_status(
        &mut self,
        key: PanelKey,
        state: PanelState,
        message: impl Into<String>,
        code: Option<String>,
    ) -> Result<(), CommandError> {
        let panel = self.get_panel_mut(key)?;
        panel.status = PanelRuntimeStatus {
            state,
            message: message.into(),
            code,
        };
        Ok(())
    }

    pub fn set_latest_frame(
        &mut self,
        key: PanelKey,
        frame: FrameCache,
    ) -> Result<(), CommandError> {
        let panel = self.get_panel_mut(key)?;
        panel.latest_frame = Some(frame);
        Ok(())
    }

    pub fn clear_latest_frame(&mut self, key: PanelKey) -> Result<(), CommandError> {
        let panel = self.get_panel_mut(key)?;
        panel.latest_frame = None;
        Ok(())
    }

    pub fn set_recording(&mut self, key: PanelKey, is_recording: bool) -> Result<(), CommandError> {
        let panel = self.get_panel_mut(key)?;
        panel.is_recording = is_recording;
        Ok(())
    }

    pub fn latest_frame(&self, key: PanelKey) -> Result<Option<FrameCache>, CommandError> {
        let panel = self.get_panel(key)?;
        Ok(panel.latest_frame.clone())
    }

    pub fn latest_frames_for_screen(
        &self,
        screen_id: u32,
    ) -> Result<Vec<(PanelKey, FrameCache)>, CommandError> {
        if screen_id as usize >= self.screens.len() {
            return Err(CommandError::config("screen_id does not exist"));
        }

        let mut frames = Vec::new();
        let screen = &self.screens[screen_id as usize];
        for panel_idx in 0..4 {
            if let Some(frame) = screen.panels[panel_idx].latest_frame.clone() {
                frames.push((
                    PanelKey {
                        screen_id,
                        panel_id: panel_idx as u8,
                    },
                    frame,
                ));
            }
        }
        Ok(frames)
    }

    pub fn effective_preview_fps_for_key(&self, key: PanelKey) -> Result<u8, CommandError> {
        self.desired_preview_fps_for_key_with_active_count(key, self.active_stream_count())
    }

    fn priority_panel_key(&self) -> Option<PanelKey> {
        if self.screens.is_empty() || self.active_screen as usize >= self.screens.len() {
            return None;
        }

        let panel_id = *self
            .active_panel_per_screen
            .get(self.active_screen as usize)?;
        let key = PanelKey {
            screen_id: self.active_screen,
            panel_id,
        };
        let panel = self.get_panel(key).ok()?;
        if !is_active_stream_state(panel.status.state) {
            return None;
        }
        if panel.config.advanced.preview_fps_override.is_some() {
            return None;
        }
        Some(key)
    }

    fn desired_preview_fps_for_key_with_active_count(
        &self,
        key: PanelKey,
        active_stream_count: usize,
    ) -> Result<u8, CommandError> {
        let panel = self.get_panel(key)?;
        if let Some(preview_fps_override) = panel.config.advanced.preview_fps_override {
            return Ok(preview_fps_override);
        }

        let priority_key = self.priority_panel_key();
        Ok(managed_preview_fps_with_priority(
            &self.stream_defaults,
            active_stream_count,
            priority_key == Some(key),
            priority_key.is_some(),
        ))
    }

    pub fn active_stream_count(&self) -> usize {
        self.screens
            .iter()
            .flat_map(|screen| screen.panels.iter())
            .filter(|panel| is_active_stream_state(panel.status.state))
            .count()
    }

    pub fn set_preview_fps_in_use(
        &mut self,
        key: PanelKey,
        preview_fps: Option<u8>,
    ) -> Result<(), CommandError> {
        let panel = self.get_panel_mut(key)?;
        panel.preview_fps_in_use = preview_fps;
        Ok(())
    }

    pub fn preview_fps_rebalance_keys(
        &self,
        exclude: Option<PanelKey>,
    ) -> Result<Vec<PanelKey>, CommandError> {
        let active_stream_count = self.active_stream_count();
        if active_stream_count == 0 {
            return Ok(Vec::new());
        }

        let mut keys = Vec::new();
        for screen in &self.screens {
            for panel_idx in 0..4 {
                let key = PanelKey {
                    screen_id: screen.id,
                    panel_id: panel_idx as u8,
                };
                if Some(key) == exclude {
                    continue;
                }

                let panel = &screen.panels[panel_idx];
                if !is_active_stream_state(panel.status.state) {
                    continue;
                }

                let desired =
                    self.desired_preview_fps_for_key_with_active_count(key, active_stream_count)?;
                if panel.preview_fps_in_use != Some(desired) {
                    keys.push(key);
                }
            }
        }

        Ok(keys)
    }

    pub fn panel_connection_tuple(&self, key: PanelKey) -> Result<ConnectionTuple, CommandError> {
        let panel = self.get_panel(key)?;
        Ok(connection_tuple(
            &panel.config,
            panel.secret.as_ref().is_some_and(PanelSecret::is_present),
        ))
    }

    pub fn playing_keys(&self) -> Vec<PanelKey> {
        let mut keys = Vec::new();
        for screen in &self.screens {
            for panel_idx in 0..4 {
                if is_active_stream_state(screen.panels[panel_idx].status.state) {
                    keys.push(PanelKey {
                        screen_id: screen.id,
                        panel_id: panel_idx as u8,
                    });
                }
            }
        }
        keys
    }

    pub fn create_screen(&mut self) -> Result<u32, CommandError> {
        if self.screens.len() >= rtsp_core::MAX_SCREEN_COUNT {
            return Err(CommandError::config("maximum screen count is 32"));
        }
        let new_id = self.screens.len() as u32;
        let cfg = default_screen_config(new_id);
        let panels = array::from_fn(|panel_idx| PanelRuntime {
            config: cfg.panels[panel_idx].clone(),
            status: PanelRuntimeStatus::default(),
            secret: None,
            preview_fps_in_use: None,
            latest_frame: None,
            is_recording: false,
        });
        self.screens.push(ScreenRuntime { id: new_id, panels });
        self.active_panel_per_screen.push(0);
        if self.screens.len() == 1 {
            self.active_screen = 0;
        }
        Ok(new_id)
    }

    pub fn delete_screen(&mut self, screen_id: u32) -> Result<(), CommandError> {
        if self.screens.is_empty() {
            return Err(CommandError::config("no screens exist"));
        }
        if screen_id as usize >= self.screens.len() {
            return Err(CommandError::config("screen_id does not exist"));
        }

        self.screens.remove(screen_id as usize);
        self.active_panel_per_screen.remove(screen_id as usize);

        if self.screens.is_empty() {
            self.active_screen = 0;
            return Ok(());
        }

        for (new_id, screen) in self.screens.iter_mut().enumerate() {
            screen.id = new_id as u32;
            for panel_idx in 0..4 {
                screen.panels[panel_idx].config.secret_ref.key =
                    secret_key_for(new_id as u32, panel_idx as u8);
            }
        }

        if self.active_screen as usize >= self.screens.len() {
            self.active_screen = self.screens.len().saturating_sub(1) as u32;
        }

        Ok(())
    }

    pub fn merge_loaded_config(
        &mut self,
        loaded: AppConfig,
        external_secrets: HashMap<String, PanelSecret>,
    ) -> Result<LoadMergeOutcome, CommandError> {
        validate_app_config(&loaded)?;

        let mut previous_playing: HashMap<PanelKey, (ConnectionTuple, u8)> = HashMap::new();
        let previous_active_stream_count = self.active_stream_count();

        for screen in &self.screens {
            for panel_idx in 0..4 {
                let key = PanelKey {
                    screen_id: screen.id,
                    panel_id: panel_idx as u8,
                };
                let panel = &screen.panels[panel_idx];
                if panel.status.state == PanelState::Playing {
                    previous_playing.insert(
                        key,
                        (
                            connection_tuple(
                                &panel.config,
                                panel.secret.as_ref().is_some_and(PanelSecret::is_present),
                            ),
                            self.desired_preview_fps_for_key_with_active_count(
                                key,
                                previous_active_stream_count,
                            )?,
                        ),
                    );
                }
            }
        }

        let mut new_runtime = Self::from_config(loaded, external_secrets);

        let mut stop_keys = Vec::new();
        let mut restart_keys = Vec::new();
        let next_active_stream_count = previous_playing
            .keys()
            .filter(|key| new_runtime.panel_exists(**key))
            .count();

        for (key, (old_tuple, old_preview_fps)) in &previous_playing {
            if !new_runtime.panel_exists(*key) {
                stop_keys.push(*key);
                continue;
            }

            let new_tuple = new_runtime.panel_connection_tuple(*key)?;
            let new_preview_fps = new_runtime
                .desired_preview_fps_for_key_with_active_count(*key, next_active_stream_count)?;
            if *old_tuple == new_tuple && *old_preview_fps == new_preview_fps {
                new_runtime.set_panel_status(*key, PanelState::Playing, "Playing", None)?;
                new_runtime.set_preview_fps_in_use(*key, Some(new_preview_fps))?;
            } else {
                stop_keys.push(*key);
                restart_keys.push(*key);
            }
        }

        *self = new_runtime;

        Ok(LoadMergeOutcome {
            stop_keys,
            restart_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtsp_core::{default_panel_config, default_screen_config, PanelConfigPatch};

    fn runtime_with_screens(count: u32) -> AppRuntimeState {
        let config = rtsp_core::AppConfig {
            schema_version: SCHEMA_VERSION,
            screens: (0..count).map(default_screen_config).collect(),
            ui_state: rtsp_core::UiState {
                active_screen: 0,
                active_panel_per_screen: vec![0; count as usize],
                fullscreen: false,
            },
            auto_populate_tool: rtsp_core::AutoPopulateTool::default(),
            stream_defaults: StreamDefaults::default(),
            saved_secrets: HashMap::new(),
        };
        AppRuntimeState::from_config(config, HashMap::new())
    }

    fn activate_panel(runtime: &mut AppRuntimeState, key: PanelKey, preview_fps: u8) {
        runtime
            .set_panel_status(key, PanelState::Playing, "Playing", None)
            .expect("status should set");
        runtime
            .set_preview_fps_in_use(key, Some(preview_fps))
            .expect("preview fps should set");
    }

    #[test]
    fn default_runtime_starts_with_zero_screens() {
        let runtime = AppRuntimeState::new_default();
        assert_eq!(runtime.screen_count(), 0);
    }

    #[test]
    fn create_and_delete_screen_updates_indices() {
        let mut runtime = runtime_with_screens(4);
        assert_eq!(runtime.screen_count(), 4);
        runtime
            .create_screen()
            .expect("create screen should succeed");
        assert_eq!(runtime.screen_count(), 5);
        runtime.delete_screen(1).expect("delete should succeed");
        assert_eq!(runtime.screen_count(), 4);
        for (idx, screen) in runtime.screens.iter().enumerate() {
            assert_eq!(screen.id, idx as u32);
            for panel_idx in 0..4 {
                assert_eq!(
                    screen.panels[panel_idx].config.secret_ref.key,
                    secret_key_for(idx as u32, panel_idx as u8)
                );
            }
        }
    }

    #[test]
    fn deleting_last_screen_is_allowed() {
        let mut runtime = runtime_with_screens(1);
        runtime.delete_screen(0).expect("delete should succeed");
        assert_eq!(runtime.screen_count(), 0);
        assert!(runtime.active_panel_per_screen.is_empty());
        assert_eq!(runtime.active_screen, 0);
    }

    #[test]
    fn update_panel_patch_changes_tuple_when_host_changes() {
        let mut runtime = runtime_with_screens(1);
        let key = PanelKey {
            screen_id: 0,
            panel_id: 0,
        };
        runtime
            .set_panel_status(key, PanelState::Playing, "Playing", None)
            .expect("status should set");
        runtime.screens[0].panels[0].config = default_panel_config(0, 0);

        let outcome = runtime
            .update_panel_config(
                key,
                PanelConfigPatch {
                    host: Some("10.0.0.5".to_string()),
                    ..PanelConfigPatch::default()
                },
            )
            .expect("patch should apply");

        assert!(outcome.was_playing);
        assert!(outcome.restart_required);
    }

    #[test]
    fn updating_preview_override_requires_restart_when_playing() {
        let mut runtime = runtime_with_screens(1);
        let key = PanelKey {
            screen_id: 0,
            panel_id: 0,
        };

        activate_panel(&mut runtime, key, 12);

        let outcome = runtime
            .update_panel_config(
                key,
                PanelConfigPatch {
                    advanced: Some(rtsp_core::AdvancedConfigPatch {
                        preview_fps_override: Some(Some(8)),
                        ..rtsp_core::AdvancedConfigPatch::default()
                    }),
                    ..PanelConfigPatch::default()
                },
            )
            .expect("patch should apply");

        assert!(outcome.was_playing);
        assert!(outcome.restart_required);
    }

    #[test]
    fn updating_stream_defaults_restarts_only_inheriting_playing_panels() {
        let mut runtime = runtime_with_screens(2);
        let inherited = PanelKey {
            screen_id: 0,
            panel_id: 0,
        };
        let overridden = PanelKey {
            screen_id: 0,
            panel_id: 1,
        };

        activate_panel(&mut runtime, inherited, 12);
        activate_panel(&mut runtime, overridden, 6);
        runtime.screens[0].panels[1]
            .config
            .advanced
            .preview_fps_override = Some(6);
        runtime.screens[1].panels[0]
            .config
            .advanced
            .preview_fps_override = Some(9);

        let outcome = runtime
            .update_stream_defaults(StreamDefaultsPatch {
                preview_fps: Some(15),
                auto_manage_preview_fps: None,
            })
            .expect("stream defaults should update");

        assert_eq!(outcome.restart_keys, vec![inherited]);
        assert_eq!(runtime.stream_defaults.preview_fps, 15);
    }

    #[test]
    fn automatic_preview_fps_rebalances_only_active_inheriting_panels() {
        let mut runtime = runtime_with_screens(2);
        runtime.stream_defaults.auto_manage_preview_fps = true;

        let inherited_a = PanelKey {
            screen_id: 0,
            panel_id: 0,
        };
        let inherited_b = PanelKey {
            screen_id: 0,
            panel_id: 1,
        };
        let overridden = PanelKey {
            screen_id: 0,
            panel_id: 2,
        };

        activate_panel(&mut runtime, inherited_a, 12);
        activate_panel(&mut runtime, inherited_b, 12);
        activate_panel(&mut runtime, overridden, 5);
        runtime.screens[0].panels[2]
            .config
            .advanced
            .preview_fps_override = Some(5);

        assert_eq!(
            runtime.preview_fps_rebalance_keys(None).unwrap(),
            Vec::<PanelKey>::new()
        );

        let inherited_c = PanelKey {
            screen_id: 0,
            panel_id: 3,
        };
        let inherited_d = PanelKey {
            screen_id: 1,
            panel_id: 0,
        };
        let inherited_e = PanelKey {
            screen_id: 1,
            panel_id: 1,
        };

        activate_panel(&mut runtime, inherited_c, 12);
        activate_panel(&mut runtime, inherited_d, 12);
        activate_panel(&mut runtime, inherited_e, 12);

        assert_eq!(
            runtime.preview_fps_rebalance_keys(None).unwrap(),
            vec![
                inherited_b,
                inherited_c,
                inherited_d,
                inherited_e
            ]
        );
    }

    #[test]
    fn auto_managed_preview_fps_prioritizes_selected_live_panel() {
        let mut runtime = runtime_with_screens(2);
        runtime.stream_defaults.auto_manage_preview_fps = true;
        runtime.stream_defaults.preview_fps = 12;

        let focused = PanelKey {
            screen_id: 0,
            panel_id: 0,
        };
        let peer_a = PanelKey {
            screen_id: 0,
            panel_id: 1,
        };
        let peer_b = PanelKey {
            screen_id: 0,
            panel_id: 2,
        };
        let peer_c = PanelKey {
            screen_id: 0,
            panel_id: 3,
        };
        let peer_d = PanelKey {
            screen_id: 1,
            panel_id: 0,
        };

        activate_panel(&mut runtime, focused, 12);
        activate_panel(&mut runtime, peer_a, 8);
        activate_panel(&mut runtime, peer_b, 8);
        activate_panel(&mut runtime, peer_c, 8);
        activate_panel(&mut runtime, peer_d, 8);

        assert_eq!(runtime.effective_preview_fps_for_key(focused).unwrap(), 12);
        assert_eq!(runtime.effective_preview_fps_for_key(peer_a).unwrap(), 8);
        assert_eq!(runtime.effective_preview_fps_for_key(peer_d).unwrap(), 8);

        runtime
            .set_active_panel(0, 1)
            .expect("active panel should update");
        assert_eq!(
            runtime.preview_fps_rebalance_keys(None).unwrap(),
            vec![focused, peer_a]
        );

        runtime
            .set_preview_fps_in_use(focused, Some(8))
            .expect("preview fps should update");
        runtime
            .set_preview_fps_in_use(peer_a, Some(12))
            .expect("preview fps should update");

        runtime
            .set_active_screen(1)
            .expect("active screen should update");
        assert_eq!(runtime.effective_preview_fps_for_key(peer_d).unwrap(), 12);
        assert_eq!(runtime.effective_preview_fps_for_key(peer_a).unwrap(), 8);
        assert_eq!(
            runtime.preview_fps_rebalance_keys(None).unwrap(),
            vec![peer_a, peer_d]
        );
    }

    #[test]
    fn merge_keeps_playing_if_tuple_unchanged() {
        let mut runtime = runtime_with_screens(1);
        let key = PanelKey {
            screen_id: 0,
            panel_id: 0,
        };

        runtime
            .update_panel_config(
                key,
                PanelConfigPatch {
                    host: Some("192.168.0.10".to_string()),
                    path: Some("stream".to_string()),
                    ..PanelConfigPatch::default()
                },
            )
            .expect("patch should apply");
        runtime
            .set_panel_status(key, PanelState::Playing, "Playing", None)
            .expect("set status should work");

        let loaded = runtime.to_app_config();
        let outcome = runtime
            .merge_loaded_config(loaded, HashMap::new())
            .expect("merge should work");
        assert!(outcome.stop_keys.is_empty());
        assert!(outcome.restart_keys.is_empty());
        assert_eq!(
            runtime.screens[0].panels[0].status.state,
            PanelState::Playing
        );
    }

    #[test]
    fn merge_loaded_config_clears_previous_secret_when_loaded_config_has_none() {
        let mut runtime = AppRuntimeState::from_config(
            default_app_config(1),
            HashMap::from([(
                "screen_0_panel_0".to_string(),
                PanelSecret {
                    username: "old-user".to_string(),
                    password: "old-pass".to_string(),
                },
            )]),
        );
        let key = PanelKey {
            screen_id: 0,
            panel_id: 0,
        };
        runtime
            .update_panel_config(
                key,
                PanelConfigPatch {
                    host: Some("127.0.0.1".to_string()),
                    path: Some("stream".to_string()),
                    ..PanelConfigPatch::default()
                },
            )
            .expect("patch should apply");
        activate_panel(&mut runtime, key, 12);

        let loaded = default_app_config(1);
        runtime
            .merge_loaded_config(loaded, HashMap::new())
            .expect("merge should work");

        assert!(runtime.screens[0].panels[0].secret.is_none());
    }
}
