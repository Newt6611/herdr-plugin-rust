use std::path::PathBuf;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{TabCloseResponse, TabCreateResponse, TabInfoResponse, TabList},
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
