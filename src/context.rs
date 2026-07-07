use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{env::HerdrEnv, events::EventKind, logger::Logger, HerdrClient};

/// Shared context passed to every plugin event handler.
pub struct Context<State = ()> {
    services: Arc<RuntimeServices<State>>,
}

struct RuntimeServices<State> {
    client: Arc<HerdrClient>,
    env: Arc<HerdrEnv>,
    state: Arc<State>,
}

impl Context<()> {
    pub fn new(client: impl Into<Arc<HerdrClient>>) -> Self {
        Self::with_env_and_state(client, HerdrEnv::from_env(), Arc::new(()))
    }
}

impl<State> Context<State> {
    pub(crate) fn with_env_and_state(
        client: impl Into<Arc<HerdrClient>>,
        env: HerdrEnv,
        state: Arc<State>,
    ) -> Self {
        Self {
            services: Arc::new(RuntimeServices {
                client: client.into(),
                env: Arc::new(env),
                state,
            }),
        }
    }

    pub fn client(&self) -> &HerdrClient {
        &self.services.client
    }

    pub fn env(&self) -> &HerdrEnv {
        &self.services.env
    }

    pub fn state(&self) -> &State {
        &self.services.state
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

    pub(crate) fn client_handle(&self) -> Arc<HerdrClient> {
        self.services.client.clone()
    }

    pub(crate) fn state_handle(&self) -> Arc<State> {
        self.services.state.clone()
    }
}

impl<State> Clone for Context<State> {
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
