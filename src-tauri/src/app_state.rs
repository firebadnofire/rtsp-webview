use crate::state::{AppRuntimeState, PanelKey};
use rtsp_secrets::{KeyringSecretStore, SecretStore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub struct StreamTask {
    pub cancel: CancellationToken,
    pub handle: JoinHandle<()>,
}

pub struct SharedState {
    pub runtime: RwLock<AppRuntimeState>,
    pub streams: Mutex<HashMap<PanelKey, StreamTask>>,
    pub secret_store: Arc<dyn SecretStore>,
}

#[derive(Clone)]
pub struct ManagedState {
    pub inner: Arc<SharedState>,
}

impl ManagedState {
    pub fn new() -> Self {
        Self::with_secret_store(Arc::new(KeyringSecretStore::default()))
    }

    pub fn with_secret_store(secret_store: Arc<dyn SecretStore>) -> Self {
        Self {
            inner: Arc::new(SharedState {
                runtime: RwLock::new(AppRuntimeState::new_default()),
                streams: Mutex::new(HashMap::new()),
                secret_store,
            }),
        }
    }
}
