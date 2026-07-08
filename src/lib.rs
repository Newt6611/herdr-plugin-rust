//! Rust SDK surface for Herdr plugins.

mod agent;
mod client;
pub mod dispatcher;
pub mod env;
mod error;
pub mod event_source;
pub mod events;
pub mod logger;
mod models;
mod pane;
mod plugin;
mod runtime;
mod session;
mod tab;
mod workspace;
mod worktree;

mod context;

use std::{
    error::Error,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};

pub use agent::{
    AgentClient, AgentExplainOptions, AgentReadOptions, AgentReadSource, AgentStartOptions,
    AgentWaitStatus, ReadFormat,
};
pub use client::{CommandLine, HerdrClient};
pub use context::Context;
pub use dispatcher::{EventDispatcher, Handler};
pub use env::{HerdrEnv, PluginInvocationContext};
pub use error::{HerdrCommandError, HerdrError};
pub use event_source::{EventSourceOutput, RuntimeEvent};
pub use events::*;
pub use logger::Logger;
pub use models::{
    AgentInfoResponse, AgentList, AgentReadResponse, DeleteSessionResponse, InstalledPluginInfo,
    Pane, PaneActionResponse, PaneCloseResponse, PaneCurrentResponse, PaneEdgesResponse,
    PaneInfoResponse, PaneLayout, PaneLayoutResponse, PaneList, PaneProcessInfoResponse, PaneRead,
    PaneRect, PaneSplit, PaneSplitPane, PluginDisableResponse, PluginEnableResponse,
    PluginListResponse, PluginPaneCloseResponse, PluginPaneFocusResponse, PluginPaneInfo,
    PluginPaneOpenResponse, PluginSourceInfo, ProcessInfo, ProcessInfoProcess, Session,
    SessionList, StopSessionResponse, Tab, TabCloseResponse, TabCreateResponse, TabInfoResponse,
    TabList, TabPane, Workspace, WorkspaceCloseResponse, WorkspaceCreateResponse,
    WorkspaceInfoResponse, WorkspaceList, WorkspacePane, WorkspaceTab, WorkspaceWorktree, Worktree,
    WorktreeCreateResponse, WorktreeList, WorktreeOpenResponse, WorktreeRemoveResponse,
    WorktreeSourceInfo,
};
pub use pane::{
    Direction, PaneClient, PaneListOptions, PaneMoveDestination, PaneMoveOptions, PaneSelector,
    PaneSplitOptions, PaneZoomMode, PluginPaneDirection, PluginPaneOpenOptions,
    PluginPanePlacement,
};
pub use plugin::{PluginClient, PluginInstallOptions, PluginListOptions};
pub use runtime::{
    OneShotRuntime, Runtime, RuntimeApp, RuntimeFuture, RuntimeHandle, RuntimeHandleError,
};
use serde::de::DeserializeOwned;
pub use session::SessionClient;
pub use tab::{TabClient, TabCreateOptions, TabListOptions};
pub use workspace::{WorkspaceClient, WorkspaceCreateOptions};
pub use worktree::{
    WorktreeClient, WorktreeCreateOptions, WorktreeListOptions, WorktreeOpenOptions,
    WorktreeOpenTarget, WorktreeSource,
};

pub type SetupError = Box<dyn Error + Send + Sync + 'static>;
pub type SetupResult<T = ()> = Result<T, SetupError>;
pub type RuntimeResult<T = ()> = Result<T, RuntimeError>;
type SetupFuture = Pin<Box<dyn Future<Output = Result<(), SetupError>> + Send + 'static>>;
type SetupHandler<State, Config> =
    Box<dyn Fn(Context<State, Config>) -> SetupFuture + Send + Sync + 'static>;
type TeardownHandler<State, Config> =
    Box<dyn Fn(Context<State, Config>) -> SetupFuture + Send + Sync + 'static>;
type ErrorFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
type ErrorHandler<State, Config> =
    Box<dyn Fn(Context<State, Config>, String) -> ErrorFuture + Send + Sync + 'static>;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("HERDR_PLUGIN_CONFIG_DIR is required to load a relative config path")]
    MissingConfigDir,
    #[error("failed to read config file at {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file at {path}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("invalid HERDR_PLUGIN_EVENT_JSON")]
    InvalidEventJson {
        json: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("setup callback failed")]
    Setup {
        #[source]
        source: SetupError,
    },
    #[error("teardown callback failed")]
    Teardown {
        #[source]
        source: SetupError,
    },
    #[error("failed to load plugin config")]
    Config {
        #[source]
        source: ConfigError,
    },
}

/// Runtime application facade used to register and dispatch typed events.
pub struct App<State, Config, AppRuntime> {
    runtime: AppRuntime,
    client: Option<Arc<HerdrClient>>,
    state: Arc<Mutex<State>>,
    config: Config,
    config_path: Option<PathBuf>,
    herdr_bin_path_override: Option<PathBuf>,
    dispatcher: EventDispatcher<Context<State, Config>>,
    setup_handlers: Vec<SetupHandler<State, Config>>,
    teardown_handlers: Vec<TeardownHandler<State, Config>>,
    error_handlers: Vec<ErrorHandler<State, Config>>,
}

impl App<(), (), OneShotRuntime> {
    /// Creates a builder for configuring and constructing an app.
    pub fn builder() -> AppBuilder<(), (), OneShotRuntime> {
        AppBuilder::new()
    }

    /// Creates an app with default builder settings.
    ///
    /// Prefer [`App::builder`] when configuration or typed config loading is needed.
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("failed to build Herdr plugin app")
    }

    /// Creates an app using an existing Herdr client handle.
    pub fn with_client(client: impl Into<Arc<HerdrClient>>) -> Self {
        Self::builder()
            .with_client(client)
            .build()
            .expect("failed to build Herdr plugin app")
    }
}

pub struct AppBuilder<State, Config, AppRuntime> {
    client: Option<Arc<HerdrClient>>,
    state: State,
    config: Config,
    config_path: Option<PathBuf>,
    herdr_bin_path_override: Option<PathBuf>,
    runtime: AppRuntime,
}

impl AppBuilder<(), (), OneShotRuntime> {
    pub fn new() -> Self {
        Self {
            client: None,
            state: (),
            config: (),
            config_path: None,
            herdr_bin_path_override: None,
            runtime: OneShotRuntime::new(),
        }
    }
}

impl Default for AppBuilder<(), (), OneShotRuntime> {
    fn default() -> Self {
        Self::new()
    }
}

impl<CurrentState, Config, AppRuntime> AppBuilder<CurrentState, Config, AppRuntime>
where
    CurrentState: Send + Sync + 'static,
    Config: Send + Sync + 'static,
{
    /// Uses an existing Herdr client handle.
    pub fn with_client(mut self, client: impl Into<Arc<HerdrClient>>) -> Self {
        self.client = Some(client.into());
        self
    }

    /// Attaches typed state that setup callbacks and event handlers can access through `Context`.
    pub fn with_state<State>(self, state: State) -> AppBuilder<State, Config, AppRuntime>
    where
        State: Send + Sync + 'static,
    {
        AppBuilder {
            client: self.client,
            state,
            config: self.config,
            config_path: self.config_path,
            herdr_bin_path_override: self.herdr_bin_path_override,
            runtime: self.runtime,
        }
    }

    /// Sets the runtime strategy that owns app lifecycle execution.
    pub fn runtime<NextRuntime>(
        self,
        runtime: NextRuntime,
    ) -> AppBuilder<CurrentState, Config, NextRuntime>
    where
        NextRuntime: Send + 'static,
    {
        AppBuilder {
            client: self.client,
            state: self.state,
            config: self.config,
            config_path: self.config_path,
            herdr_bin_path_override: self.herdr_bin_path_override,
            runtime,
        }
    }

    /// Loads typed plugin config from `$HERDR_PLUGIN_CONFIG_DIR/config.toml`.
    ///
    /// Missing config files use `Default::default()`. Invalid TOML returns
    /// [`RuntimeError::Config`].
    pub fn with_config<NextConfig>(self) -> AppBuilder<CurrentState, NextConfig, AppRuntime>
    where
        NextConfig: DeserializeOwned + Default + Send + Sync + 'static,
    {
        self.with_config_file("config.toml")
    }

    /// Loads typed plugin config from a custom TOML path.
    ///
    /// Relative paths are resolved under `HERDR_PLUGIN_CONFIG_DIR`; absolute
    /// paths are used as-is.
    pub fn with_config_file<NextConfig>(
        self,
        path: impl Into<PathBuf>,
    ) -> AppBuilder<CurrentState, NextConfig, AppRuntime>
    where
        NextConfig: DeserializeOwned + Default + Send + Sync + 'static,
    {
        let path = path.into();
        AppBuilder {
            client: self.client,
            state: self.state,
            config: NextConfig::default(),
            config_path: Some(path),
            herdr_bin_path_override: self.herdr_bin_path_override,
            runtime: self.runtime,
        }
    }

    /// Alias for [`AppBuilder::with_config_file`] when a call site wants to emphasize
    /// that the argument may be absolute.
    pub fn with_config_path<NextConfig>(
        self,
        path: impl Into<PathBuf>,
    ) -> AppBuilder<CurrentState, NextConfig, AppRuntime>
    where
        NextConfig: DeserializeOwned + Default + Send + Sync + 'static,
    {
        self.with_config_file(path)
    }

    /// Sets the Herdr binary path used by the client passed to handlers.
    pub fn with_herdr_bin_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        self.client = Some(Arc::new(HerdrClient::with_binary(path.clone())));
        self.herdr_bin_path_override = Some(path.clone());
        self
    }

    /// Reads the current Herdr environment, loads configured services, and returns a ready app.
    pub fn build(self) -> Result<App<CurrentState, Config, AppRuntime>, RuntimeError>
    where
        Config: DeserializeOwned + Default,
        AppRuntime: Runtime<CurrentState, Config>,
    {
        Ok(App {
            runtime: self.runtime,
            client: self.client,
            state: Arc::new(Mutex::new(self.state)),
            config: self.config,
            config_path: self.config_path,
            herdr_bin_path_override: self.herdr_bin_path_override,
            dispatcher: EventDispatcher::default(),
            setup_handlers: Vec::new(),
            teardown_handlers: Vec::new(),
            error_handlers: Vec::new(),
        })
    }
}

impl<State, Config, AppRuntime> App<State, Config, AppRuntime>
where
    State: Send + Sync + 'static,
    Config: DeserializeOwned + Default + Send + Sync + 'static,
    AppRuntime: Runtime<State, Config>,
{
    /// Registers an async handler for a concrete event type.
    pub fn on<E>(&mut self, handler: impl Handler<Context<State, Config>, E>) -> &mut Self
    where
        E: Event,
    {
        self.dispatcher.on(handler);
        self
    }

    /// Registers an async event handler and returns the app for builder chaining.
    pub fn on_event<E>(mut self, handler: impl Handler<Context<State, Config>, E>) -> Self
    where
        E: Event,
    {
        self.on(handler);
        self
    }

    /// Registers a setup callback that runs before the current event starts dispatching.
    ///
    /// Setup callbacks run before event dispatch and receive the same [`Context`]
    /// as event handlers.
    pub fn setup<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(Context<State, Config>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SetupError>> + Send + 'static,
    {
        self.setup_handlers
            .push(Box::new(move |context| Box::pin(handler(context))));
        self
    }

    /// Registers a teardown callback that runs after event dispatch completes.
    pub fn teardown<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(Context<State, Config>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SetupError>> + Send + 'static,
    {
        self.teardown_handlers
            .push(Box::new(move |context| Box::pin(handler(context))));
        self
    }

    /// Registers a callback that runs before `run` returns a runtime error.
    pub fn on_error<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(Context<State, Config>, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.error_handlers.push(Box::new(move |context, error| {
            Box::pin(handler(context, error))
        }));
        self
    }

    /// Runs the app for the current Herdr plugin invocation.
    pub async fn run(self) -> Result<(), RuntimeError> {
        let runtime = self.runtime;
        let app = RuntimeApp::new(
            self.client,
            self.state,
            self.config,
            self.config_path,
            self.herdr_bin_path_override,
            self.dispatcher,
            self.setup_handlers,
            self.teardown_handlers,
            self.error_handlers,
        );
        runtime.run(app).await
    }
}

impl Default for App<(), (), OneShotRuntime> {
    fn default() -> Self {
        Self::new()
    }
}

fn load_config<Config>(env: &HerdrEnv, path: &Path) -> Result<Config, ConfigError>
where
    Config: DeserializeOwned + Default,
{
    let path = resolve_config_path(env, path)?;
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default())
        }
        Err(source) => return Err(ConfigError::Read { path, source }),
    };

    toml::from_str(&contents).map_err(|source| ConfigError::Parse { path, source })
}

fn resolve_config_path(env: &HerdrEnv, path: &Path) -> Result<PathBuf, ConfigError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    env.plugin_config_dir
        .as_ref()
        .map(|dir| dir.join(path))
        .ok_or(ConfigError::MissingConfigDir)
}
