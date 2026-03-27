use rtsp_core::{PanelConfig, PanelState};

pub trait MediaBackend {
    fn prepare_stream(&self, _config: &PanelConfig) -> Result<(), MediaError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("media backend is not implemented")]
    NotImplemented,
}

pub struct StubMediaBackend;

impl MediaBackend for StubMediaBackend {
    fn prepare_stream(&self, _config: &PanelConfig) -> Result<(), MediaError> {
        Ok(())
    }
}

pub fn terminal_state() -> PanelState {
    PanelState::Stopped
}
