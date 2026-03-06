use crate::errors::CommandError;
use rtsp_core::{
    apply_panel_patch, connection_tuple, default_app_config, default_screen_config, secret_key_for,
    validate_app_config, AppConfig, AutoPopulateTool, ConnectionTuple, GetStateResponse,
    PanelConfig, PanelConfigPatch, PanelRuntimeStatus, PanelState, PanelStateView, ScreenStateView,
    DEFAULT_SCREEN_COUNT, SCHEMA_VERSION,
};
use std::array;
use std::collections::HashMap;

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
    pub data_base64: String,
}

#[derive(Debug, Clone)]
pub struct PanelRuntime {
    pub config: PanelConfig,
    pub status: PanelRuntimeStatus,
    pub secret: Option<PanelSecret>,
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
}

#[derive(Debug, Clone)]
pub struct UpdatePanelOutcome {
    pub was_playing: bool,
    pub tuple_changed: bool,
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
        let panel = self.get_panel_mut(key)?;
        let old_tuple = connection_tuple(
            &panel.config,
            panel.secret.as_ref().is_some_and(PanelSecret::is_present),
        );
        let was_playing = panel.status.state == PanelState::Playing;
        apply_panel_patch(&mut panel.config, patch)?;
        let new_tuple = connection_tuple(
            &panel.config,
            panel.secret.as_ref().is_some_and(PanelSecret::is_present),
        );
        Ok(UpdatePanelOutcome {
            was_playing,
            tuple_changed: old_tuple != new_tuple,
        })
    }

    pub fn set_auto_populate_tool_value(&mut self, value: AutoPopulateTool) {
        self.auto_populate_tool = value;
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
        let was_playing = panel.status.state == PanelState::Playing;

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

    pub fn set_recording(&mut self, key: PanelKey, is_recording: bool) -> Result<(), CommandError> {
        let panel = self.get_panel_mut(key)?;
        panel.is_recording = is_recording;
        Ok(())
    }

    pub fn latest_frame(&self, key: PanelKey) -> Result<Option<FrameCache>, CommandError> {
        let panel = self.get_panel(key)?;
        Ok(panel.latest_frame.clone())
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
                if screen.panels[panel_idx].status.state == PanelState::Playing {
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

        let mut previous_secret_map: HashMap<String, PanelSecret> = HashMap::new();
        let mut previous_playing: HashMap<PanelKey, ConnectionTuple> = HashMap::new();

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
                        connection_tuple(
                            &panel.config,
                            panel.secret.as_ref().is_some_and(PanelSecret::is_present),
                        ),
                    );
                }
                if let Some(secret) = &panel.secret {
                    previous_secret_map.insert(panel.config.secret_ref.key.clone(), secret.clone());
                }
            }
        }

        for (key, value) in external_secrets {
            previous_secret_map.entry(key).or_insert(value);
        }

        let mut new_runtime = Self::from_config(loaded, previous_secret_map);

        let mut stop_keys = Vec::new();
        let mut restart_keys = Vec::new();

        for (key, old_tuple) in &previous_playing {
            if !new_runtime.panel_exists(*key) {
                stop_keys.push(*key);
                continue;
            }

            let new_tuple = new_runtime.panel_connection_tuple(*key)?;
            if *old_tuple == new_tuple {
                new_runtime.set_panel_status(*key, PanelState::Playing, "Playing", None)?;
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
        };
        AppRuntimeState::from_config(config, HashMap::new())
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
        assert!(outcome.tuple_changed);
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
}
