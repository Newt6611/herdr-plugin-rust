use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{DeleteSessionResponse, SessionList, StopSessionResponse},
};

#[derive(Clone, Copy, Debug)]
pub struct SessionClient<'a> {
    client: &'a HerdrClient,
}

impl<'a> SessionClient<'a> {
    pub(crate) fn new(client: &'a HerdrClient) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<SessionList, HerdrError> {
        self.client.run_json(["session", "list", "--json"]).await
    }

    pub async fn attach(&self, name: &str) -> Result<(), HerdrError> {
        self.client.run_empty(["session", "attach", name]).await
    }

    pub async fn stop(&self, name: &str) -> Result<StopSessionResponse, HerdrError> {
        self.client
            .run_json(["session", "stop", name, "--json"])
            .await
    }

    pub async fn delete(&self, name: &str) -> Result<DeleteSessionResponse, HerdrError> {
        self.client
            .run_json(["session", "delete", name, "--json"])
            .await
    }
}
