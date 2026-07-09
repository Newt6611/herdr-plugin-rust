use std::path::PathBuf;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{
        WorkspaceCloseResponse, WorkspaceCreateResponse, WorkspaceInfoResponse, WorkspaceList,
    },
    RuntimeHandle, RuntimeHandleError,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceCreateOptions {
    pub cwd: Option<PathBuf>,
    pub label: Option<String>,
    pub env: Vec<(String, String)>,
    pub focus: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceClient<'a> {
    client: &'a HerdrClient,
}

impl<'a> WorkspaceClient<'a> {
    pub(crate) fn new(client: &'a HerdrClient) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<WorkspaceList, HerdrError> {
        self.client.run_json_result(["workspace", "list"]).await
    }

    pub async fn create(
        &self,
        options: WorkspaceCreateOptions,
    ) -> Result<WorkspaceCreateResponse, HerdrError> {
        let mut args = vec!["workspace".to_owned(), "create".to_owned()];

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

    pub async fn get(&self, workspace_id: &str) -> Result<WorkspaceInfoResponse, HerdrError> {
        self.client
            .run_json_result(["workspace", "get", workspace_id])
            .await
    }

    pub async fn focus(&self, workspace_id: &str) -> Result<WorkspaceInfoResponse, HerdrError> {
        self.client
            .run_json_result(["workspace", "focus", workspace_id])
            .await
    }

    pub async fn rename(
        &self,
        workspace_id: &str,
        label: &str,
    ) -> Result<WorkspaceInfoResponse, HerdrError> {
        self.client
            .run_json_result(["workspace", "rename", workspace_id, label])
            .await
    }

    pub async fn close(&self, workspace_id: &str) -> Result<WorkspaceCloseResponse, HerdrError> {
        self.client
            .run_json_result(["workspace", "close", workspace_id])
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketWorkspaceClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketWorkspaceClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn list(&self) -> Result<WorkspaceList, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:workspace:list",
                "workspace.list",
                json_object(),
            )
            .await
    }

    pub async fn create(
        &self,
        options: WorkspaceCreateOptions,
    ) -> Result<WorkspaceCreateResponse, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        if let Some(cwd) = options.cwd {
            params.insert(
                "cwd".to_owned(),
                serde_json::Value::String(cwd.display().to_string()),
            );
        }
        if let Some(label) = options.label {
            params.insert("label".to_owned(), serde_json::Value::String(label));
        }
        if !options.env.is_empty() {
            params.insert(
                "env".to_owned(),
                serde_json::Value::Object(
                    options
                        .env
                        .into_iter()
                        .map(|(key, value)| (key, serde_json::Value::String(value)))
                        .collect(),
                ),
            );
        }
        if let Some(focus) = options.focus {
            params.insert("focus".to_owned(), serde_json::Value::Bool(focus));
        }

        self.handle
            .request_json_result(
                "herdr-plugin:workspace:create",
                "workspace.create",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:workspace:get",
                "workspace.get",
                serde_json::json!({ "workspace_id": workspace_id }),
            )
            .await
    }

    pub async fn focus(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:workspace:focus",
                "workspace.focus",
                serde_json::json!({ "workspace_id": workspace_id }),
            )
            .await
    }

    pub async fn rename(
        &self,
        workspace_id: &str,
        label: &str,
    ) -> Result<WorkspaceInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:workspace:rename",
                "workspace.rename",
                serde_json::json!({ "workspace_id": workspace_id, "label": label }),
            )
            .await
    }

    pub async fn close(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceCloseResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:workspace:close",
                "workspace.close",
                serde_json::json!({ "workspace_id": workspace_id }),
            )
            .await
    }
}

impl RuntimeHandle {
    pub fn workspace(&self) -> SocketWorkspaceClient<'_> {
        SocketWorkspaceClient::new(self)
    }
}

fn json_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}
