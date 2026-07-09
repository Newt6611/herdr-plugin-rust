use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{
        PluginDisableResponse, PluginEnableResponse, PluginListResponse, PluginPaneCloseResponse,
        PluginPaneFocusResponse, PluginPaneOpenResponse,
    },
    pane::PluginPaneOpenOptions,
    RuntimeHandle, RuntimeHandleError,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginLinkOptions {
    pub path: String,
    pub enabled: bool,
    pub source: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginActionListOptions {
    pub plugin_id: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginLogListOptions {
    pub plugin_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginActionInvokeOptions {
    pub action_id: String,
    pub plugin_id: Option<String>,
    pub context: Option<serde_json::Value>,
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

#[derive(Clone, Copy, Debug)]
pub struct SocketPluginClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketPluginClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn link(
        &self,
        options: PluginLinkOptions,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        params.insert("path".to_owned(), serde_json::Value::String(options.path));
        params.insert(
            "enabled".to_owned(),
            serde_json::Value::Bool(options.enabled),
        );
        if let Some(source) = options.source {
            params.insert("source".to_owned(), source);
        }
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:link",
                "plugin.link",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn list(
        &self,
        options: PluginListOptions,
    ) -> Result<PluginListResponse, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        if let Some(plugin_id) = options.plugin_id {
            params.insert("plugin_id".to_owned(), serde_json::Value::String(plugin_id));
        }
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:list",
                "plugin.list",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn unlink(&self, plugin_id: &str) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:unlink",
                "plugin.unlink",
                serde_json::json!({ "plugin_id": plugin_id }),
            )
            .await
    }

    pub async fn enable(
        &self,
        plugin_id: &str,
    ) -> Result<PluginEnableResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:enable",
                "plugin.enable",
                serde_json::json!({ "plugin_id": plugin_id }),
            )
            .await
    }

    pub async fn disable(
        &self,
        plugin_id: &str,
    ) -> Result<PluginDisableResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:disable",
                "plugin.disable",
                serde_json::json!({ "plugin_id": plugin_id }),
            )
            .await
    }

    pub async fn action_list(
        &self,
        options: PluginActionListOptions,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        if let Some(plugin_id) = options.plugin_id {
            params.insert("plugin_id".to_owned(), serde_json::Value::String(plugin_id));
        }
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:action:list",
                "plugin.action.list",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn action_invoke(
        &self,
        options: PluginActionInvokeOptions,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        params.insert(
            "action_id".to_owned(),
            serde_json::Value::String(options.action_id),
        );
        if let Some(plugin_id) = options.plugin_id {
            params.insert("plugin_id".to_owned(), serde_json::Value::String(plugin_id));
        }
        if let Some(context) = options.context {
            params.insert("context".to_owned(), context);
        }
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:action:invoke",
                "plugin.action.invoke",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn log_list(
        &self,
        options: PluginLogListOptions,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        if let Some(plugin_id) = options.plugin_id {
            params.insert("plugin_id".to_owned(), serde_json::Value::String(plugin_id));
        }
        if let Some(limit) = options.limit {
            params.insert(
                "limit".to_owned(),
                serde_json::Value::Number((limit as u64).into()),
            );
        }
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:log:list",
                "plugin.log.list",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn pane_open(
        &self,
        options: PluginPaneOpenOptions,
    ) -> Result<PluginPaneOpenResponse, RuntimeHandleError> {
        self.handle.pane().open_plugin_pane(options).await
    }

    pub async fn pane_focus(
        &self,
        pane_id: &str,
    ) -> Result<PluginPaneFocusResponse, RuntimeHandleError> {
        self.handle.pane().focus_plugin_pane(pane_id).await
    }

    pub async fn pane_close(
        &self,
        pane_id: &str,
    ) -> Result<PluginPaneCloseResponse, RuntimeHandleError> {
        self.handle.pane().close_plugin_pane(pane_id).await
    }
}

impl RuntimeHandle {
    pub fn plugin(&self) -> SocketPluginClient<'_> {
        SocketPluginClient::new(self)
    }
}
