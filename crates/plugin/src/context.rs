use std::sync::Arc;

use crate::{env::HerdrEnv, HerdrClient};

/// Shared context passed to every plugin event handler.
#[derive(Clone)]
pub struct Context {
    services: Arc<RuntimeServices>,
}

struct RuntimeServices {
    client: Arc<HerdrClient>,
    env: Arc<HerdrEnv>,
}

impl Context {
    pub fn new(client: impl Into<Arc<HerdrClient>>) -> Self {
        Self::with_env(client, HerdrEnv::from_env())
    }

    pub(crate) fn with_env(client: impl Into<Arc<HerdrClient>>, env: HerdrEnv) -> Self {
        Self {
            services: Arc::new(RuntimeServices {
                client: client.into(),
                env: Arc::new(env),
            }),
        }
    }

    pub fn client(&self) -> &HerdrClient {
        &self.services.client
    }

    pub fn env(&self) -> &HerdrEnv {
        &self.services.env
    }

    pub(crate) fn client_handle(&self) -> Arc<HerdrClient> {
        self.services.client.clone()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(HerdrClient::new())
    }
}
