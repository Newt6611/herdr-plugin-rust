use std::sync::Arc;

use crate::{env::HerdrEnv, HerdrClient};

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
