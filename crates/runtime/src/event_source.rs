use herdr_dispatcher::EventDispatcher;

use crate::{
    context::Context,
    env::HerdrEnv,
    events::{
        EventData, EventEnvelope, PaneAgentDetected, PaneAgentStatusChanged, PaneClosed,
        PaneCreated, PaneExited, PaneFocused, PaneMoved, PaneOutputChanged, TabClosed, TabCreated,
        TabFocused, TabMoved, TabRenamed, WorkspaceClosed, WorkspaceCreated, WorkspaceFocused,
        WorkspaceMoved, WorkspaceRenamed, WorkspaceUpdated,
    },
    RuntimeError,
};

#[derive(Debug, Clone)]
pub struct EventSourceOutput {
    pub env: HerdrEnv,
    pub event: Option<RuntimeEvent>,
}

pub struct EnvEventSource;

impl EnvEventSource {
    pub fn from_env() -> Result<EventSourceOutput, RuntimeError> {
        let mut env = HerdrEnv::from_env();
        let Some(json) = env.plugin_event_json_raw.clone() else {
            return Ok(EventSourceOutput { env, event: None });
        };
        let envelope: EventEnvelope = serde_json::from_str(&json)
            .map_err(|source| RuntimeError::InvalidEventJson { json, source })?;
        env.plugin_event_json = Some(envelope.clone());

        Ok(EventSourceOutput {
            env,
            event: Some(RuntimeEvent::from(envelope)),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    WorkspaceCreated(WorkspaceCreated),
    WorkspaceUpdated(WorkspaceUpdated),
    WorkspaceClosed(WorkspaceClosed),
    WorkspaceRenamed(WorkspaceRenamed),
    WorkspaceMoved(WorkspaceMoved),
    WorkspaceFocused(WorkspaceFocused),
    TabCreated(TabCreated),
    TabClosed(TabClosed),
    TabRenamed(TabRenamed),
    TabMoved(TabMoved),
    TabFocused(TabFocused),
    PaneCreated(PaneCreated),
    PaneClosed(PaneClosed),
    PaneFocused(PaneFocused),
    PaneMoved(PaneMoved),
    PaneOutputChanged(PaneOutputChanged),
    PaneExited(PaneExited),
    PaneAgentDetected(PaneAgentDetected),
    PaneAgentStatusChanged(PaneAgentStatusChanged),
}

impl RuntimeEvent {
    pub(crate) async fn dispatch(self, dispatcher: &EventDispatcher<Context>, context: Context) {
        match self {
            Self::WorkspaceCreated(event) => dispatcher.dispatch(context, event).await,
            Self::WorkspaceUpdated(event) => dispatcher.dispatch(context, event).await,
            Self::WorkspaceClosed(event) => dispatcher.dispatch(context, event).await,
            Self::WorkspaceRenamed(event) => dispatcher.dispatch(context, event).await,
            Self::WorkspaceMoved(event) => dispatcher.dispatch(context, event).await,
            Self::WorkspaceFocused(event) => dispatcher.dispatch(context, event).await,
            Self::TabCreated(event) => dispatcher.dispatch(context, event).await,
            Self::TabClosed(event) => dispatcher.dispatch(context, event).await,
            Self::TabRenamed(event) => dispatcher.dispatch(context, event).await,
            Self::TabMoved(event) => dispatcher.dispatch(context, event).await,
            Self::TabFocused(event) => dispatcher.dispatch(context, event).await,
            Self::PaneCreated(event) => dispatcher.dispatch(context, event).await,
            Self::PaneClosed(event) => dispatcher.dispatch(context, event).await,
            Self::PaneFocused(event) => dispatcher.dispatch(context, event).await,
            Self::PaneMoved(event) => dispatcher.dispatch(context, event).await,
            Self::PaneOutputChanged(event) => dispatcher.dispatch(context, event).await,
            Self::PaneExited(event) => dispatcher.dispatch(context, event).await,
            Self::PaneAgentDetected(event) => dispatcher.dispatch(context, event).await,
            Self::PaneAgentStatusChanged(event) => dispatcher.dispatch(context, event).await,
        }
    }
}

impl From<EventEnvelope> for RuntimeEvent {
    fn from(envelope: EventEnvelope) -> Self {
        match envelope.data {
            EventData::WorkspaceCreated { workspace } => {
                Self::WorkspaceCreated(WorkspaceCreated { workspace })
            }
            EventData::WorkspaceUpdated { workspace } => {
                Self::WorkspaceUpdated(WorkspaceUpdated { workspace })
            }
            EventData::WorkspaceClosed {
                workspace_id,
                workspace,
            } => Self::WorkspaceClosed(WorkspaceClosed {
                workspace_id,
                workspace,
            }),
            EventData::WorkspaceRenamed {
                workspace_id,
                label,
            } => Self::WorkspaceRenamed(WorkspaceRenamed {
                workspace_id,
                label,
            }),
            EventData::WorkspaceMoved {
                workspace_id,
                insert_index,
                workspaces,
            } => Self::WorkspaceMoved(WorkspaceMoved {
                workspace_id,
                insert_index,
                workspaces,
            }),
            EventData::WorkspaceFocused { workspace_id } => {
                Self::WorkspaceFocused(WorkspaceFocused { workspace_id })
            }
            EventData::TabCreated { tab } => Self::TabCreated(TabCreated { tab }),
            EventData::TabClosed {
                tab_id,
                workspace_id,
            } => Self::TabClosed(TabClosed {
                tab_id,
                workspace_id,
            }),
            EventData::TabRenamed {
                tab_id,
                workspace_id,
                label,
            } => Self::TabRenamed(TabRenamed {
                tab_id,
                workspace_id,
                label,
            }),
            EventData::TabMoved {
                tab_id,
                workspace_id,
                insert_index,
                tabs,
            } => Self::TabMoved(TabMoved {
                tab_id,
                workspace_id,
                insert_index,
                tabs,
            }),
            EventData::TabFocused {
                tab_id,
                workspace_id,
            } => Self::TabFocused(TabFocused {
                tab_id,
                workspace_id,
            }),
            EventData::PaneCreated { pane } => Self::PaneCreated(PaneCreated { pane }),
            EventData::PaneClosed {
                pane_id,
                workspace_id,
            } => Self::PaneClosed(PaneClosed {
                pane_id,
                workspace_id,
            }),
            EventData::PaneFocused {
                pane_id,
                workspace_id,
            } => Self::PaneFocused(PaneFocused {
                pane_id,
                workspace_id,
            }),
            EventData::PaneMoved {
                previous_pane_id,
                previous_workspace_id,
                previous_tab_id,
                pane,
                created_workspace,
                created_tab,
                closed_workspace_id,
                closed_tab_id,
            } => Self::PaneMoved(PaneMoved {
                previous_pane_id,
                previous_workspace_id,
                previous_tab_id,
                pane: *pane,
                created_workspace,
                created_tab,
                closed_workspace_id,
                closed_tab_id,
            }),
            EventData::PaneOutputChanged {
                pane_id,
                workspace_id,
                revision,
            } => Self::PaneOutputChanged(PaneOutputChanged {
                pane_id,
                workspace_id,
                revision,
            }),
            EventData::PaneExited {
                pane_id,
                workspace_id,
            } => Self::PaneExited(PaneExited {
                pane_id,
                workspace_id,
            }),
            EventData::PaneAgentDetected {
                pane_id,
                workspace_id,
                agent,
            } => Self::PaneAgentDetected(PaneAgentDetected {
                pane_id,
                workspace_id,
                agent,
            }),
            EventData::PaneAgentStatusChanged {
                pane_id,
                workspace_id,
                agent_status,
                agent,
                title,
                display_agent,
                custom_status,
                state_labels,
            } => Self::PaneAgentStatusChanged(PaneAgentStatusChanged {
                pane_id,
                workspace_id,
                agent_status,
                agent,
                title,
                display_agent,
                custom_status,
                state_labels,
            }),
        }
    }
}
