use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{PluginDisableResponse, PluginEnableResponse, PluginListResponse},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInstallOptions {
    pub source: String,
    pub requested_ref: Option<String>,
    pub yes: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginListOptions {
    pub plugin_id: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct PluginClient<'a> {
    client: &'a HerdrClient,
}

impl<'a> PluginClient<'a> {
    pub(crate) fn new(client: &'a HerdrClient) -> Self {
        Self { client }
    }

    pub async fn install(&self, options: PluginInstallOptions) -> Result<(), HerdrError> {
        let mut args = vec!["plugin".to_owned(), "install".to_owned(), options.source];
        if let Some(requested_ref) = options.requested_ref {
            args.push("--ref".to_owned());
            args.push(requested_ref);
        }
        if options.yes {
            args.push("--yes".to_owned());
        }
        self.client.run_empty(args).await
    }

    pub async fn list(&self, options: PluginListOptions) -> Result<PluginListResponse, HerdrError> {
        let mut args = vec!["plugin".to_owned(), "list".to_owned(), "--json".to_owned()];
        if let Some(plugin_id) = options.plugin_id {
            args.push("--plugin".to_owned());
            args.push(plugin_id);
        }
        self.client.run_json_result(args).await
    }

    pub async fn uninstall(&self, target: &str) -> Result<(), HerdrError> {
        self.client.run_empty(["plugin", "uninstall", target]).await
    }

    pub async fn enable(&self, plugin_id: &str) -> Result<PluginEnableResponse, HerdrError> {
        self.client
            .run_json_result(["plugin", "enable", plugin_id])
            .await
    }

    pub async fn disable(&self, plugin_id: &str) -> Result<PluginDisableResponse, HerdrError> {
        self.client
            .run_json_result(["plugin", "disable", plugin_id])
            .await
    }
}
