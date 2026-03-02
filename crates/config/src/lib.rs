use rtsp_core::{validate_app_config, AppConfig, ValidationError};

pub fn validate(config: &AppConfig) -> Result<(), ValidationError> {
    validate_app_config(config)
}
