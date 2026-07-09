use std::path::PathBuf;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{TabCloseResponse, TabCreateResponse, TabInfoResponse, TabList},
    socket::{env_object, insert_opt, insert_opt_bool, insert_opt_path},
    RuntimeHandle, RuntimeHandleError,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TabListOptions {
    pub workspace_id: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TabCreateOptions {
    pub workspace_id: Option<String>,
    pub cwd: Option<PathBuf>,
    pub label: Option<String>,
    pub env: Vec<(String, String)>,
    pub focus: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct TabClient<'a> {
    client: &'a HerdrClient,
}

impl<'a> TabClient<'a> {
    pub(crate) fn new(client: &'a HerdrClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, options: TabListOptions) -> Result<TabList, HerdrError> {
        let mut args = vec!["tab".to_owned(), "list".to_owned()];

        if let Some(workspace_id) = options.workspace_id {
            args.push("--workspace".to_owned());
            args.push(workspace_id);
        }

        self.client.run_json_result(args).await
    }

    pub async fn create(&self, options: TabCreateOptions) -> Result<TabCreateResponse, HerdrError> {
        let mut args = vec!["tab".to_owned(), "create".to_owned()];

        if let Some(workspace_id) = options.workspace_id {
            args.push("--workspace".to_owned());
            args.push(workspace_id);
        }

        if let Some(cwd) = options.cwd {
            args.push("--cwd".to_owned());
            args.push(cwd.display().to_string());
        }

        if let Some(label) = options.label {
            args.push("--label".to_owned());
            args.push(label);
        }

        for (key, value) in options.env {
            args.push("--env".to_owned());
            args.push(format!("{key}={value}"));
        }

        match options.focus {
            Some(true) => args.push("--focus".to_owned()),
            Some(false) => args.push("--no-focus".to_owned()),
            None => {}
        }

        self.client.run_json_result(args).await
    }

    pub async fn get(&self, tab_id: &str) -> Result<TabInfoResponse, HerdrError> {
        self.client.run_json_result(["tab", "get", tab_id]).await
    }

    pub async fn focus(&self, tab_id: &str) -> Result<TabInfoResponse, HerdrError> {
        self.client.run_json_result(["tab", "focus", tab_id]).await
    }

    pub async fn rename(&self, tab_id: &str, label: &str) -> Result<TabInfoResponse, HerdrError> {
        self.client
            .run_json_result(["tab", "rename", tab_id, label])
            .await
    }

    pub async fn close(&self, tab_id: &str) -> Result<TabCloseResponse, HerdrError> {
        self.client.run_json_result(["tab", "close", tab_id]).await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketTabClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketTabClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn list(&self, options: TabListOptions) -> Result<TabList, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        insert_opt(&mut params, "workspace_id", options.workspace_id);
        self.handle
            .request_json_result(
                "herdr-plugin:tab:list",
                "tab.list",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn create(
        &self,
        options: TabCreateOptions,
    ) -> Result<TabCreateResponse, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        insert_opt(&mut params, "workspace_id", options.workspace_id);
        insert_opt_path(&mut params, "cwd", options.cwd);
        insert_opt(&mut params, "label", options.label);
        if !options.env.is_empty() {
            params.insert("env".to_owned(), env_object(options.env));
        }
        insert_opt_bool(&mut params, "focus", options.focus);
        self.handle
            .request_json_result(
                "herdr-plugin:tab:create",
                "tab.create",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn get(&self, tab_id: &str) -> Result<TabInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:tab:get",
                "tab.get",
                serde_json::json!({ "tab_id": tab_id }),
            )
            .await
    }

    pub async fn focus(&self, tab_id: &str) -> Result<TabInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:tab:focus",
                "tab.focus",
                serde_json::json!({ "tab_id": tab_id }),
            )
            .await
    }

    pub async fn rename(
        &self,
        tab_id: &str,
        label: &str,
    ) -> Result<TabInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:tab:rename",
                "tab.rename",
                serde_json::json!({ "tab_id": tab_id, "label": label }),
            )
            .await
    }

    pub async fn move_tab(
        &self,
        tab_id: &str,
        insert_index: usize,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:tab:move",
                "tab.move",
                serde_json::json!({ "tab_id": tab_id, "insert_index": insert_index }),
            )
            .await
    }

    pub async fn close(&self, tab_id: &str) -> Result<TabCloseResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:tab:close",
                "tab.close",
                serde_json::json!({ "tab_id": tab_id }),
            )
            .await
    }
}

impl RuntimeHandle {
    pub fn tab(&self) -> SocketTabClient<'_> {
        SocketTabClient::new(self)
    }
}
