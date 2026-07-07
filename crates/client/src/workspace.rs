use std::path::PathBuf;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{
        WorkspaceCloseResponse, WorkspaceCreateResponse, WorkspaceInfoResponse, WorkspaceList,
    },
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
