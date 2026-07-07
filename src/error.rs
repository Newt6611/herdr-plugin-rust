use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

use crate::client::CommandLine;

#[derive(Debug, Error)]
pub enum HerdrError {
    #[error("missing herdr executable at {binary}")]
    MissingExecutable {
        binary: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to execute herdr command: {binary} {}", args.join(" "))]
    CommandExecutionFailed {
        binary: PathBuf,
        args: Vec<String>,
        #[source]
        source: std::io::Error,
    },

    #[error("herdr command failed with status {status:?}: {error}")]
    CommandFailed {
        command: CommandLine,
        status: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        error: HerdrCommandError,
    },

    #[error("herdr command returned invalid JSON: {command}")]
    InvalidJson {
        command: CommandLine,
        stdout: Vec<u8>,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq)]
#[error("{code}: {message}")]
pub struct HerdrCommandError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HerdrCommandErrorBody {
    pub error: HerdrCommandError,
}
