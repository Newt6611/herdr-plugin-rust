//! Runtime app facade for Herdr plugins.

pub mod env;
pub mod events;

use std::{path::PathBuf, sync::Arc};

use herdr_client::HerdrClient;
use herdr_dispatcher::{EventDispatcher, Handler};

pub use env::{HerdrEnv, PluginInvocationContext};
use events::{
    EventData, EventEnvelope, PaneAgentDetected, PaneAgentStatusChanged, PaneClosed, PaneCreated,
    PaneExited, PaneFocused, PaneMoved, PaneOutputChanged, TabClosed, TabCreated, TabFocused,
    TabMoved, TabRenamed, WorkspaceClosed, WorkspaceCreated, WorkspaceFocused, WorkspaceMoved,
    WorkspaceRenamed, WorkspaceUpdated,
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

/// Shared context passed to every plugin event handler.
#[derive(Clone)]
pub struct Context {
    pub client: Arc<HerdrClient>,
    pub env: HerdrEnv,
}

impl Context {
    pub fn new(client: impl Into<Arc<HerdrClient>>) -> Self {
        Self {
            client: client.into(),
            env: HerdrEnv::from_env(),
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(HerdrClient::new())
    }
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
        self.context.client = Arc::new(HerdrClient::with_binary(path.clone()));
        self.herdr_bin_path_override = Some(path.clone());
        self.context.env.bin_path = Some(path);
        self
    }

    /// Registers an async handler for a concrete event type.
    pub fn on<E>(&mut self, handler: impl Handler<Context, E>) -> &mut Self
    where
        E: Clone + Send + Sync + 'static,
    {
        self.dispatcher.on(handler);
        self
    }

    /// Registers an async event handler and returns the app for builder chaining.
    pub fn on_event<E>(mut self, handler: impl Handler<Context, E>) -> Self
    where
        E: Clone + Send + Sync + 'static,
    {
        self.on(handler);
        self
    }

    /// Runs the app for the current Herdr plugin invocation.
    pub async fn run(mut self) -> Result<(), RuntimeError> {
        self.context.env = HerdrEnv::from_env();
        if let Some(path) = self.herdr_bin_path_override.as_ref() {
            self.context.env.bin_path = Some(path.clone());
        }

        let Some(json) = self.context.env.plugin_event_json_raw.clone() else {
            return Ok(());
        };
        let envelope: EventEnvelope = serde_json::from_str(&json)
            .map_err(|source| RuntimeError::InvalidEventJson { json, source })?;
        self.context.env.plugin_event_json = Some(envelope.clone());
        self.dispatch_envelope(envelope).await;
        Ok(())
    }

    async fn dispatch<E>(&self, event: E)
    where
        E: Clone + Send + Sync + 'static,
    {
        self.dispatcher.dispatch(self.context.clone(), event).await;
    }

    async fn dispatch_envelope(&self, envelope: EventEnvelope) {
        match envelope.data {
            EventData::WorkspaceCreated { workspace } => {
                self.dispatch(WorkspaceCreated { workspace }).await
            }
            EventData::WorkspaceUpdated { workspace } => {
                self.dispatch(WorkspaceUpdated { workspace }).await
            }
            EventData::WorkspaceClosed {
                workspace_id,
                workspace,
            } => {
                self.dispatch(WorkspaceClosed {
                    workspace_id,
                    workspace,
                })
                .await
            }
            EventData::WorkspaceRenamed {
                workspace_id,
                label,
            } => {
                self.dispatch(WorkspaceRenamed {
                    workspace_id,
                    label,
                })
                .await
            }
            EventData::WorkspaceMoved {
                workspace_id,
                insert_index,
                workspaces,
            } => {
                self.dispatch(WorkspaceMoved {
                    workspace_id,
                    insert_index,
                    workspaces,
                })
                .await
            }
            EventData::WorkspaceFocused { workspace_id } => {
                self.dispatch(WorkspaceFocused { workspace_id }).await
            }
            EventData::TabCreated { tab } => self.dispatch(TabCreated { tab }).await,
            EventData::TabClosed {
                tab_id,
                workspace_id,
            } => {
                self.dispatch(TabClosed {
                    tab_id,
                    workspace_id,
                })
                .await
            }
            EventData::TabRenamed {
                tab_id,
                workspace_id,
                label,
            } => {
                self.dispatch(TabRenamed {
                    tab_id,
                    workspace_id,
                    label,
                })
                .await
            }
            EventData::TabMoved {
                tab_id,
                workspace_id,
                insert_index,
                tabs,
            } => {
                self.dispatch(TabMoved {
                    tab_id,
                    workspace_id,
                    insert_index,
                    tabs,
                })
                .await
            }
            EventData::TabFocused {
                tab_id,
                workspace_id,
            } => {
                self.dispatch(TabFocused {
                    tab_id,
                    workspace_id,
                })
                .await
            }
            EventData::PaneCreated { pane } => self.dispatch(PaneCreated { pane }).await,
            EventData::PaneClosed {
                pane_id,
                workspace_id,
            } => {
                self.dispatch(PaneClosed {
                    pane_id,
                    workspace_id,
                })
                .await
            }
            EventData::PaneFocused {
                pane_id,
                workspace_id,
            } => {
                self.dispatch(PaneFocused {
                    pane_id,
                    workspace_id,
                })
                .await
            }
            EventData::PaneMoved {
                previous_pane_id,
                previous_workspace_id,
                previous_tab_id,
                pane,
                created_workspace,
                created_tab,
                closed_workspace_id,
                closed_tab_id,
            } => {
                self.dispatch(PaneMoved {
                    previous_pane_id,
                    previous_workspace_id,
                    previous_tab_id,
                    pane: *pane,
                    created_workspace,
                    created_tab,
                    closed_workspace_id,
                    closed_tab_id,
                })
                .await
            }
            EventData::PaneOutputChanged {
                pane_id,
                workspace_id,
                revision,
            } => {
                self.dispatch(PaneOutputChanged {
                    pane_id,
                    workspace_id,
                    revision,
                })
                .await
            }
            EventData::PaneExited {
                pane_id,
                workspace_id,
            } => {
                self.dispatch(PaneExited {
                    pane_id,
                    workspace_id,
                })
                .await
            }
            EventData::PaneAgentDetected {
                pane_id,
                workspace_id,
                agent,
            } => {
                self.dispatch(PaneAgentDetected {
                    pane_id,
                    workspace_id,
                    agent,
                })
                .await
            }
            EventData::PaneAgentStatusChanged {
                pane_id,
                workspace_id,
                agent_status,
                agent,
                title,
                display_agent,
                custom_status,
                state_labels,
            } => {
                self.dispatch(PaneAgentStatusChanged {
                    pane_id,
                    workspace_id,
                    agent_status,
                    agent,
                    title,
                    display_agent,
                    custom_status,
                    state_labels,
                })
                .await
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
