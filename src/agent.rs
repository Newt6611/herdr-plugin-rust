use std::path::PathBuf;

use serde_json::Value;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{AgentInfoResponse, AgentList, AgentReadResponse},
    pane::Direction,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentReadSource {
    Visible,
    Recent,
    RecentUnwrapped,
    Detection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadFormat {
    Text,
    Ansi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentWaitStatus {
    Idle,
    Working,
    Blocked,
    Unknown,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentReadOptions {
    pub source: Option<AgentReadSource>,
    pub lines: Option<u64>,
    pub format: Option<ReadFormat>,
    pub ansi: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentStartOptions {
    pub name: String,
    pub cwd: Option<PathBuf>,
    pub workspace_id: Option<String>,
    pub tab_id: Option<String>,
    pub split: Option<Direction>,
    pub env: Vec<(String, String)>,
    pub focus: Option<bool>,
    pub argv: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentExplainOptions {
    Target {
        target: String,
        json: bool,
        verbose: bool,
    },
    File {
        path: PathBuf,
        agent: String,
        json: bool,
        verbose: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct AgentClient<'a> {
    client: &'a HerdrClient,
}

impl<'a> AgentClient<'a> {
    pub(crate) fn new(client: &'a HerdrClient) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<AgentList, HerdrError> {
        self.client.run_json_result(["agent", "list"]).await
    }

    pub async fn get(&self, target: &str) -> Result<AgentInfoResponse, HerdrError> {
        self.client.run_json_result(["agent", "get", target]).await
    }

    pub async fn read(
        &self,
        target: &str,
        options: AgentReadOptions,
    ) -> Result<AgentReadResponse, HerdrError> {
        let mut args = vec!["agent".to_owned(), "read".to_owned(), target.to_owned()];
        if let Some(source) = options.source {
            args.push("--source".to_owned());
            args.push(source.as_str().to_owned());
        }
        if let Some(lines) = options.lines {
            args.push("--lines".to_owned());
            args.push(lines.to_string());
        }
        if let Some(format) = options.format {
            args.push("--format".to_owned());
            args.push(format.as_str().to_owned());
        }
        if options.ansi {
            args.push("--ansi".to_owned());
        }
        self.client.run_json_result(args).await
    }

    pub async fn send(&self, target: &str, text: &str) -> Result<(), HerdrError> {
        self.client.run_empty(["agent", "send", target, text]).await
    }

    pub async fn rename(
        &self,
        target: &str,
        name: Option<&str>,
    ) -> Result<AgentInfoResponse, HerdrError> {
        let mut args = vec!["agent".to_owned(), "rename".to_owned(), target.to_owned()];
        if let Some(name) = name {
            args.push(name.to_owned());
        } else {
            args.push("--clear".to_owned());
        }
        self.client.run_json_result(args).await
    }

    pub async fn focus(&self, target: &str) -> Result<AgentInfoResponse, HerdrError> {
        self.client
            .run_json_result(["agent", "focus", target])
            .await
    }

    pub async fn wait(
        &self,
        target: &str,
        status: AgentWaitStatus,
        timeout_ms: Option<u64>,
    ) -> Result<(), HerdrError> {
        let mut args = vec![
            "agent".to_owned(),
            "wait".to_owned(),
            target.to_owned(),
            "--status".to_owned(),
            status.as_str().to_owned(),
        ];
        if let Some(timeout_ms) = timeout_ms {
            args.push("--timeout".to_owned());
            args.push(timeout_ms.to_string());
        }
        self.client.run_empty(args).await
    }

    pub async fn attach(&self, target: &str, takeover: bool) -> Result<(), HerdrError> {
        let mut args = vec!["agent".to_owned(), "attach".to_owned(), target.to_owned()];
        if takeover {
            args.push("--takeover".to_owned());
        }
        self.client.run_empty(args).await
    }

    pub async fn start(&self, options: AgentStartOptions) -> Result<AgentInfoResponse, HerdrError> {
        let mut args = vec!["agent".to_owned(), "start".to_owned(), options.name];
        if let Some(cwd) = options.cwd {
            args.push("--cwd".to_owned());
            args.push(cwd.display().to_string());
        }
        if let Some(workspace_id) = options.workspace_id {
            args.push("--workspace".to_owned());
            args.push(workspace_id);
        }
        if let Some(tab_id) = options.tab_id {
            args.push("--tab".to_owned());
            args.push(tab_id);
        }
        if let Some(split) = options.split {
            args.push("--split".to_owned());
            args.push(split.as_str().to_owned());
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
        args.push("--".to_owned());
        args.extend(options.argv);
        self.client.run_json_result(args).await
    }

    pub async fn explain(&self, options: AgentExplainOptions) -> Result<Value, HerdrError> {
        let mut args = vec!["agent".to_owned(), "explain".to_owned()];
        match options {
            AgentExplainOptions::Target {
                target,
                json,
                verbose,
            } => {
                args.push(target);
                if json {
                    args.push("--json".to_owned());
                }
                if verbose {
                    args.push("--verbose".to_owned());
                }
            }
            AgentExplainOptions::File {
                path,
                agent,
                json,
                verbose,
            } => {
                args.push("--file".to_owned());
                args.push(path.display().to_string());
                args.push("--agent".to_owned());
                args.push(agent);
                if json {
                    args.push("--json".to_owned());
                }
                if verbose {
                    args.push("--verbose".to_owned());
                }
            }
        }
        self.client.run_json(args).await
    }
}

impl AgentReadSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Recent => "recent",
            Self::RecentUnwrapped => "recent-unwrapped",
            Self::Detection => "detection",
        }
    }
}

impl ReadFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Ansi => "ansi",
        }
    }
}

impl AgentWaitStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }
}
