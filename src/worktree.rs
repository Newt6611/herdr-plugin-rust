use std::path::PathBuf;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{WorktreeCreateResponse, WorktreeList, WorktreeOpenResponse, WorktreeRemoveResponse},
    socket::{insert_opt, insert_opt_bool, insert_opt_path},
    RuntimeHandle, RuntimeHandleError,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorktreeListOptions {
    pub source: Option<WorktreeSource>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorktreeCreateOptions {
    pub source: Option<WorktreeSource>,
    pub branch: Option<String>,
    pub base: Option<String>,
    pub path: Option<PathBuf>,
    pub label: Option<String>,
    pub focus: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeOpenOptions {
    pub source: Option<WorktreeSource>,
    pub target: WorktreeOpenTarget,
    pub label: Option<String>,
    pub focus: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorktreeSource {
    Workspace(String),
    Cwd(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorktreeOpenTarget {
    Path(PathBuf),
    Branch(String),
}

#[derive(Clone, Copy, Debug)]
pub struct WorktreeClient<'a> {
    client: &'a HerdrClient,
}

impl<'a> WorktreeClient<'a> {
    pub(crate) fn new(client: &'a HerdrClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, options: WorktreeListOptions) -> Result<WorktreeList, HerdrError> {
        let mut args = vec!["worktree".to_owned(), "list".to_owned()];
        push_source(&mut args, options.source);
        args.push("--json".to_owned());

        self.client.run_json_result(args).await
    }

    pub async fn create(
        &self,
        options: WorktreeCreateOptions,
    ) -> Result<WorktreeCreateResponse, HerdrError> {
        let mut args = vec!["worktree".to_owned(), "create".to_owned()];
        push_source(&mut args, options.source);

        if let Some(branch) = options.branch {
            args.push("--branch".to_owned());
            args.push(branch);
        }

        if let Some(base) = options.base {
            args.push("--base".to_owned());
            args.push(base);
        }

        if let Some(path) = options.path {
            args.push("--path".to_owned());
            args.push(path.display().to_string());
        }

        if let Some(label) = options.label {
            args.push("--label".to_owned());
            args.push(label);
        }

        push_focus(&mut args, options.focus);
        args.push("--json".to_owned());

        self.client.run_json_result(args).await
    }

    pub async fn open(
        &self,
        options: WorktreeOpenOptions,
    ) -> Result<WorktreeOpenResponse, HerdrError> {
        let mut args = vec!["worktree".to_owned(), "open".to_owned()];
        push_source(&mut args, options.source);

        match options.target {
            WorktreeOpenTarget::Path(path) => {
                args.push("--path".to_owned());
                args.push(path.display().to_string());
            }
            WorktreeOpenTarget::Branch(branch) => {
                args.push("--branch".to_owned());
                args.push(branch);
            }
        }

        if let Some(label) = options.label {
            args.push("--label".to_owned());
            args.push(label);
        }

        push_focus(&mut args, options.focus);
        args.push("--json".to_owned());

        self.client.run_json_result(args).await
    }

    pub async fn remove(
        &self,
        workspace_id: &str,
        force: bool,
    ) -> Result<WorktreeRemoveResponse, HerdrError> {
        let mut args = vec![
            "worktree".to_owned(),
            "remove".to_owned(),
            "--workspace".to_owned(),
            workspace_id.to_owned(),
        ];

        if force {
            args.push("--force".to_owned());
        }

        args.push("--json".to_owned());

        self.client.run_json_result(args).await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketWorktreeClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketWorktreeClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn list(
        &self,
        options: WorktreeListOptions,
    ) -> Result<WorktreeList, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:worktree:list",
                "worktree.list",
                worktree_source_params(options.source),
            )
            .await
    }

    pub async fn create(
        &self,
        options: WorktreeCreateOptions,
    ) -> Result<WorktreeCreateResponse, RuntimeHandleError> {
        let mut params = source_map(options.source);
        insert_opt(&mut params, "branch", options.branch);
        insert_opt(&mut params, "base", options.base);
        insert_opt_path(&mut params, "path", options.path);
        insert_opt(&mut params, "label", options.label);
        insert_opt_bool(&mut params, "focus", options.focus);
        self.handle
            .request_json_result(
                "herdr-plugin:worktree:create",
                "worktree.create",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn open(
        &self,
        options: WorktreeOpenOptions,
    ) -> Result<WorktreeOpenResponse, RuntimeHandleError> {
        let mut params = source_map(options.source);
        match options.target {
            WorktreeOpenTarget::Path(path) => insert_opt_path(&mut params, "path", Some(path)),
            WorktreeOpenTarget::Branch(branch) => insert_opt(&mut params, "branch", Some(branch)),
        }
        insert_opt(&mut params, "label", options.label);
        insert_opt_bool(&mut params, "focus", options.focus);
        self.handle
            .request_json_result(
                "herdr-plugin:worktree:open",
                "worktree.open",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn remove(
        &self,
        workspace_id: &str,
        force: bool,
    ) -> Result<WorktreeRemoveResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:worktree:remove",
                "worktree.remove",
                serde_json::json!({ "workspace_id": workspace_id, "force": force }),
            )
            .await
    }
}

impl RuntimeHandle {
    pub fn worktree(&self) -> SocketWorktreeClient<'_> {
        SocketWorktreeClient::new(self)
    }
}

fn worktree_source_params(source: Option<WorktreeSource>) -> serde_json::Value {
    serde_json::Value::Object(source_map(source))
}

fn source_map(source: Option<WorktreeSource>) -> serde_json::Map<String, serde_json::Value> {
    let mut params = serde_json::Map::new();
    match source {
        Some(WorktreeSource::Workspace(workspace_id)) => {
            params.insert(
                "workspace_id".to_owned(),
                serde_json::Value::String(workspace_id),
            );
        }
        Some(WorktreeSource::Cwd(cwd)) => {
            params.insert(
                "cwd".to_owned(),
                serde_json::Value::String(cwd.display().to_string()),
            );
        }
        None => {}
    }
    params
}

fn push_source(args: &mut Vec<String>, source: Option<WorktreeSource>) {
    match source {
        Some(WorktreeSource::Workspace(workspace_id)) => {
            args.push("--workspace".to_owned());
            args.push(workspace_id);
        }
        Some(WorktreeSource::Cwd(cwd)) => {
            args.push("--cwd".to_owned());
            args.push(cwd.display().to_string());
        }
        None => {}
    }
}

fn push_focus(args: &mut Vec<String>, focus: Option<bool>) {
    match focus {
        Some(true) => args.push("--focus".to_owned()),
        Some(false) => args.push("--no-focus".to_owned()),
        None => {}
    }
}
