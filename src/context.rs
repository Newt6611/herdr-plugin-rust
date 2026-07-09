use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{env::HerdrEnv, events::EventKind, logger::Logger, HerdrClient, RuntimeHandle};

/// Shared context passed to every plugin callback and event handler.
pub struct Context<State = (), Config = ()> {
    services: Arc<RuntimeServices<State, Config>>,
}

struct RuntimeServices<State, Config> {
    client: Arc<HerdrClient>,
    env: Arc<HerdrEnv>,
    state: Arc<Mutex<State>>,
    config: Arc<Config>,
    socket: Option<RuntimeHandle>,
}

impl Context<()> {
    pub fn new(client: impl Into<Arc<HerdrClient>>) -> Self {
        Self::with_env_state_and_config(
            client,
            HerdrEnv::from_env(),
            Arc::new(Mutex::new(())),
            Arc::new(()),
        )
    }
}

impl<State, Config> Context<State, Config> {
    pub(crate) fn with_env_state_and_config(
        client: impl Into<Arc<HerdrClient>>,
        env: HerdrEnv,
        state: Arc<Mutex<State>>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            services: Arc::new(RuntimeServices {
                client: client.into(),
                env: Arc::new(env),
                state,
                config,
                socket: None,
            }),
        }
    }

    pub(crate) fn with_socket(self, socket: RuntimeHandle) -> Self {
        Self {
            services: Arc::new(RuntimeServices {
                client: self.services.client.clone(),
                env: self.services.env.clone(),
                state: self.services.state.clone(),
                config: self.services.config.clone(),
                socket: Some(socket),
            }),
        }
    }

    pub fn client(&self) -> &HerdrClient {
        &self.services.client
    }

    pub fn socket(&self) -> Option<RuntimeHandle> {
        self.services.socket.clone()
    }

    pub fn env(&self) -> &HerdrEnv {
        &self.services.env
    }

    pub fn state(&self) -> MutexGuard<'_, State> {
        self.services.state.lock().expect("state mutex poisoned")
    }

    pub fn state_mut(&self) -> MutexGuard<'_, State> {
        self.services.state.lock().expect("state mutex poisoned")
    }

    pub fn config(&self) -> &Config {
        &self.services.config
    }

    pub fn log(&self) -> Logger<'_> {
        Logger::new(self.env())
    }

    pub fn is_event(&self) -> bool {
        self.services.env.plugin_event_json.is_some()
    }

    pub fn is_action(&self) -> bool {
        self.services.env.plugin_action_id.is_some()
    }

    pub fn event_kind(&self) -> Option<EventKind> {
        self.services
            .env
            .plugin_event_json
            .as_ref()
            .map(|event| event.event)
    }

    pub fn config_dir(&self) -> Option<&Path> {
        self.services.env.plugin_config_dir.as_deref()
    }

    pub fn state_dir(&self) -> Option<&Path> {
        self.services.env.plugin_state_dir.as_deref()
    }

    pub fn config_path(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        self.config_dir().map(|dir| dir.join(path))
    }

    pub fn state_path(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        self.state_dir().map(|dir| dir.join(path))
    }
}

impl<State, Config> Clone for Context<State, Config> {
    fn clone(&self) -> Self {
        Self {
            services: self.services.clone(),
        }
    }
}

impl Default for Context<()> {
    fn default() -> Self {
        Self::new(HerdrClient::new())
    }
}
