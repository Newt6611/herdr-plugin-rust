//! Rust SDK surface for Herdr plugins.

mod agent;
mod client;
pub mod dispatcher;
pub mod env;
mod error;
pub mod event_source;
pub mod events;
mod models;
mod pane;
mod session;
mod tab;
mod workspace;
mod worktree;

mod context;

use std::{path::PathBuf, sync::Arc};

pub use agent::{
    AgentClient, AgentExplainOptions, AgentReadOptions, AgentReadSource, AgentStartOptions,
    AgentWaitStatus, ReadFormat,
};
pub use client::{CommandLine, HerdrClient};
pub use context::Context;
pub use dispatcher::{EventDispatcher, Handler};
pub use env::{HerdrEnv, PluginInvocationContext};
use event_source::EnvEventSource;
pub use event_source::{EventSourceOutput, RuntimeEvent};
pub use events::*;
pub use models::{
    AgentInfoResponse, AgentList, AgentReadResponse, DeleteSessionResponse, Pane,
    PaneActionResponse, PaneCloseResponse, PaneCurrentResponse, PaneEdgesResponse,
    PaneInfoResponse, PaneLayout, PaneLayoutResponse, PaneList, PaneProcessInfoResponse, PaneRead,
    PaneRect, PaneSplit, PaneSplitPane, ProcessInfo, ProcessInfoProcess, Session, SessionList,
    StopSessionResponse, Tab, TabCloseResponse, TabCreateResponse, TabInfoResponse, TabList,
    TabPane, Workspace, WorkspaceCloseResponse, WorkspaceCreateResponse, WorkspaceInfoResponse,
    WorkspaceList, WorkspacePane, WorkspaceTab, WorkspaceWorktree, Worktree,
    WorktreeCreateResponse, WorktreeList, WorktreeOpenResponse, WorktreeRemoveResponse,
    WorktreeSourceInfo,
};
pub use pane::{
    Direction, PaneClient, PaneListOptions, PaneMoveDestination, PaneMoveOptions, PaneSelector,
    PaneSplitOptions, PaneZoomMode,
};
pub use session::SessionClient;
pub use tab::{TabClient, TabCreateOptions, TabListOptions};
pub use workspace::{WorkspaceClient, WorkspaceCreateOptions};
pub use worktree::{
    WorktreeClient, WorktreeCreateOptions, WorktreeListOptions, WorktreeOpenOptions,
    WorktreeOpenTarget, WorktreeSource,
};

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("invalid HERDR_PLUGIN_EVENT_JSON")]
    InvalidEventJson {
        json: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Runtime application facade used to register and dispatch typed events.
pub struct App {
    context: Context,
    dispatcher: EventDispatcher<Context>,
    herdr_bin_path_override: Option<PathBuf>,
}

impl App {
    /// Creates an app.
    pub fn new() -> Self {
        Self::with_client(HerdrClient::new())
    }

    /// Creates an app using an existing Herdr client handle.
    pub fn with_client(client: impl Into<Arc<HerdrClient>>) -> Self {
        Self {
            context: Context::new(client),
            dispatcher: EventDispatcher::default(),
            herdr_bin_path_override: None,
        }
    }

    /// Sets the Herdr binary path used by the client passed to event handlers.
    pub fn with_herdr_bin_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut env = self.context.env().clone();
        let client = Arc::new(HerdrClient::with_binary(path.clone()));
        self.herdr_bin_path_override = Some(path.clone());
        env.bin_path = Some(path);
        self.context = Context::with_env(client, env);
        self
    }

    /// Registers an async handler for a concrete event type.
    pub fn on<E>(&mut self, handler: impl Handler<Context, E>) -> &mut Self
    where
        E: Event,
    {
        self.dispatcher.on(handler);
        self
    }

    /// Registers an async event handler and returns the app for builder chaining.
    pub fn on_event<E>(mut self, handler: impl Handler<Context, E>) -> Self
    where
        E: Event,
    {
        self.on(handler);
        self
    }

    /// Runs the app for the current Herdr plugin invocation.
    pub async fn run(mut self) -> Result<(), RuntimeError> {
        let mut output = EnvEventSource::from_env()?;
        if let Some(path) = self.herdr_bin_path_override.as_ref() {
            output.env.bin_path = Some(path.clone());
        }

        self.context = Context::with_env(self.context.client_handle(), output.env);
        if let Some(event) = output.event {
            event.dispatch(&self.dispatcher, self.context.clone()).await;
        }
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// A Herdr plugin module that registers event handlers on an application.
pub trait Plugin {
    fn build(&self, app: &mut App);
}
