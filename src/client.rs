use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use tokio::process::Command;

use crate::{
    agent::AgentClient,
    error::{HerdrCommandError, HerdrCommandErrorBody, HerdrError},
    pane::PaneClient,
    session::SessionClient,
    tab::TabClient,
    workspace::WorkspaceClient,
    worktree::WorktreeClient,
};

#[derive(Clone, Debug)]
pub struct HerdrClient {
    binary: PathBuf,
}

impl HerdrClient {
    pub fn new() -> Self {
        Self::with_binary(std::env::var_os("HERDR_BIN_PATH").unwrap_or_else(|| "herdr".into()))
    }

    pub fn with_binary(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    pub fn session(&self) -> SessionClient<'_> {
        SessionClient::new(self)
    }

    pub fn workspace(&self) -> WorkspaceClient<'_> {
        WorkspaceClient::new(self)
    }

    pub fn worktree(&self) -> WorktreeClient<'_> {
        WorktreeClient::new(self)
    }

    pub fn tab(&self) -> TabClient<'_> {
        TabClient::new(self)
    }

    pub fn pane(&self) -> PaneClient<'_> {
        PaneClient::new(self)
    }

    pub fn agent(&self) -> AgentClient<'_> {
        AgentClient::new(self)
    }

    pub(crate) async fn run_json<T>(
        &self,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<T, HerdrError>
    where
        T: DeserializeOwned,
    {
        let output = self.output(args).await?;
        let command = output.command;

        if !output.status.success() {
            let error = parse_command_error(&output.stdout, &output.stderr);
            return Err(HerdrError::CommandFailed {
                command,
                status: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
                error,
            });
        }

        serde_json::from_slice(&output.stdout).map_err(|source| HerdrError::InvalidJson {
            command,
            stdout: output.stdout,
            source,
        })
    }

    pub(crate) async fn run_empty(
        &self,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<(), HerdrError> {
        let output = self.output(args).await?;
        let command = output.command;

        if !output.status.success() {
            let error = parse_command_error(&output.stdout, &output.stderr);
            return Err(HerdrError::CommandFailed {
                command,
                status: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
                error,
            });
        }

        Ok(())
    }

    pub(crate) async fn run_json_result<T>(
        &self,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<T, HerdrError>
    where
        T: DeserializeOwned,
    {
        let response = self.run_json::<CliResponse<T>>(args).await?;
        Ok(response.result)
    }

    async fn output(
        &self,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Result<CommandOutput, HerdrError> {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        let output = Command::new(&self.binary)
            .args(&args)
            .output()
            .await
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::NotFound {
                    HerdrError::MissingExecutable {
                        binary: self.binary.clone(),
                        source,
                    }
                } else {
                    HerdrError::CommandExecutionFailed {
                        binary: self.binary.clone(),
                        args: args.clone(),
                        source,
                    }
                }
            })?;

        Ok(CommandOutput {
            command: CommandLine::new(&self.binary, args),
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
struct CliResponse<T> {
    result: T,
}

impl Default for HerdrClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct CommandOutput {
    command: CommandLine,
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandLine {
    binary: PathBuf,
    args: Vec<String>,
}

impl CommandLine {
    fn new(binary: &Path, args: Vec<String>) -> Self {
        Self {
            binary: binary.to_path_buf(),
            args,
        }
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }
}

impl std::fmt::Display for CommandLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.binary.display())?;
        for arg in &self.args {
            write!(formatter, " {arg}")?;
        }
        Ok(())
    }
}

fn parse_command_error(stdout: &[u8], stderr: &[u8]) -> HerdrCommandError {
    if let Ok(body) = serde_json::from_slice::<HerdrCommandErrorBody>(stdout) {
        return body.error;
    }

    let message = String::from_utf8_lossy(stderr).trim().to_owned();
    let message = if message.is_empty() {
        String::from_utf8_lossy(stdout).trim().to_owned()
    } else {
        message
    };

    HerdrCommandError {
        code: "command_failed".to_owned(),
        message,
    }
}
