use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    WorkspaceCreated,
    WorkspaceUpdated,
    WorkspaceClosed,
    WorkspaceRenamed,
    WorkspaceMoved,
    WorkspaceFocused,
    TabCreated,
    TabClosed,
    TabRenamed,
    TabMoved,
    TabFocused,
    PaneCreated,
    PaneClosed,
    PaneFocused,
    PaneMoved,
    PaneOutputChanged,
    PaneExited,
    PaneAgentDetected,
    PaneAgentStatusChanged,
}

impl EventKind {
    pub fn dot_name(self) -> &'static str {
        match self {
            Self::WorkspaceCreated => "workspace.created",
            Self::WorkspaceUpdated => "workspace.updated",
            Self::WorkspaceClosed => "workspace.closed",
            Self::WorkspaceRenamed => "workspace.renamed",
            Self::WorkspaceMoved => "workspace.moved",
            Self::WorkspaceFocused => "workspace.focused",
            Self::TabCreated => "tab.created",
            Self::TabClosed => "tab.closed",
            Self::TabRenamed => "tab.renamed",
            Self::TabMoved => "tab.moved",
            Self::TabFocused => "tab.focused",
            Self::PaneCreated => "pane.created",
            Self::PaneClosed => "pane.closed",
            Self::PaneFocused => "pane.focused",
            Self::PaneMoved => "pane.moved",
            Self::PaneOutputChanged => "pane.output_changed",
            Self::PaneExited => "pane.exited",
            Self::PaneAgentDetected => "pane.agent_detected",
            Self::PaneAgentStatusChanged => "pane.agent_status_changed",
        }
    }
}

pub trait HerdrEvent {
    const KIND: EventKind;

    fn dot_name() -> &'static str {
        Self::KIND.dot_name()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event: EventKind,
    pub data: EventData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventData {
    WorkspaceCreated {
        workspace: WorkspaceInfo,
    },
    WorkspaceUpdated {
        workspace: WorkspaceInfo,
    },
    WorkspaceClosed {
        workspace_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace: Option<WorkspaceInfo>,
    },
    WorkspaceRenamed {
        workspace_id: String,
        label: String,
    },
    WorkspaceMoved {
        workspace_id: String,
        insert_index: usize,
        workspaces: Vec<WorkspaceInfo>,
    },
    WorkspaceFocused {
        workspace_id: String,
    },
    TabCreated {
        tab: TabInfo,
    },
    TabClosed {
        tab_id: String,
        workspace_id: String,
    },
    TabRenamed {
        tab_id: String,
        workspace_id: String,
        label: String,
    },
    TabMoved {
        tab_id: String,
        workspace_id: String,
        insert_index: usize,
        tabs: Vec<TabInfo>,
    },
    TabFocused {
        tab_id: String,
        workspace_id: String,
    },
    PaneCreated {
        pane: PaneInfo,
    },
    PaneClosed {
        pane_id: String,
        workspace_id: String,
    },
    PaneFocused {
        pane_id: String,
        workspace_id: String,
    },
    PaneMoved {
        previous_pane_id: String,
        previous_workspace_id: String,
        previous_tab_id: String,
        pane: Box<PaneInfo>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        created_workspace: Option<WorkspaceInfo>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        created_tab: Option<TabInfo>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        closed_workspace_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        closed_tab_id: Option<String>,
    },
    PaneOutputChanged {
        pane_id: String,
        workspace_id: String,
        revision: u64,
    },
    PaneExited {
        pane_id: String,
        workspace_id: String,
    },
    PaneAgentDetected {
        pane_id: String,
        workspace_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent: Option<String>,
    },
    PaneAgentStatusChanged {
        pane_id: String,
        workspace_id: String,
        agent_status: AgentStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_agent: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        custom_status: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        state_labels: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Done,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub workspace_id: String,
    pub number: usize,
    pub label: String,
    pub focused: bool,
    pub pane_count: usize,
    pub tab_count: usize,
    pub active_tab_id: String,
    pub agent_status: AgentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorkspaceWorktreeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceWorktreeInfo {
    pub repo_key: String,
    pub repo_name: String,
    pub repo_root: String,
    pub checkout_path: String,
    pub is_linked_worktree: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabInfo {
    pub tab_id: String,
    pub workspace_id: String,
    pub number: usize,
    pub label: String,
    pub focused: bool,
    pub pane_count: usize,
    pub agent_status: AgentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionInfo {
    pub source: String,
    pub agent: String,
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneInfo {
    pub pane_id: String,
    pub terminal_id: String,
    pub workspace_id: String,
    pub tab_id: String,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground_cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_agent: Option<String>,
    pub agent_status: AgentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_status: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub state_labels: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_session: Option<AgentSessionInfo>,
    pub revision: u64,
}

macro_rules! event_struct {
    ($name:ident, $kind:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct $name {
            $(pub $field: $ty,)*
        }

        impl HerdrEvent for $name {
            const KIND: EventKind = EventKind::$kind;
        }
    };
}

event_struct!(
    WorkspaceCreated,
    WorkspaceCreated {
        workspace: WorkspaceInfo
    }
);
event_struct!(
    WorkspaceUpdated,
    WorkspaceUpdated {
        workspace: WorkspaceInfo
    }
);
event_struct!(WorkspaceClosed, WorkspaceClosed {
    workspace_id: String,
    workspace: Option<WorkspaceInfo>,
});
event_struct!(
    WorkspaceRenamed,
    WorkspaceRenamed {
        workspace_id: String,
        label: String,
    }
);
event_struct!(WorkspaceMoved, WorkspaceMoved {
    workspace_id: String,
    insert_index: usize,
    workspaces: Vec<WorkspaceInfo>,
});
event_struct!(
    WorkspaceFocused,
    WorkspaceFocused {
        workspace_id: String
    }
);

event_struct!(TabCreated, TabCreated { tab: TabInfo });
event_struct!(
    TabClosed,
    TabClosed {
        tab_id: String,
        workspace_id: String,
    }
);
event_struct!(
    TabRenamed,
    TabRenamed {
        tab_id: String,
        workspace_id: String,
        label: String,
    }
);
event_struct!(TabMoved, TabMoved {
    tab_id: String,
    workspace_id: String,
    insert_index: usize,
    tabs: Vec<TabInfo>,
});
event_struct!(
    TabFocused,
    TabFocused {
        tab_id: String,
        workspace_id: String,
    }
);

event_struct!(PaneCreated, PaneCreated { pane: PaneInfo });
event_struct!(
    PaneClosed,
    PaneClosed {
        pane_id: String,
        workspace_id: String,
    }
);
event_struct!(
    PaneFocused,
    PaneFocused {
        pane_id: String,
        workspace_id: String,
    }
);
event_struct!(PaneMoved, PaneMoved {
    previous_pane_id: String,
    previous_workspace_id: String,
    previous_tab_id: String,
    pane: PaneInfo,
    created_workspace: Option<WorkspaceInfo>,
    created_tab: Option<TabInfo>,
    closed_workspace_id: Option<String>,
    closed_tab_id: Option<String>,
});
event_struct!(
    PaneOutputChanged,
    PaneOutputChanged {
        pane_id: String,
        workspace_id: String,
        revision: u64,
    }
);
event_struct!(
    PaneExited,
    PaneExited {
        pane_id: String,
        workspace_id: String,
    }
);
event_struct!(PaneAgentDetected, PaneAgentDetected {
    pane_id: String,
    workspace_id: String,
    agent: Option<String>,
});
event_struct!(PaneAgentStatusChanged, PaneAgentStatusChanged {
    pane_id: String,
    workspace_id: String,
    agent_status: AgentStatus,
    agent: Option<String>,
    title: Option<String>,
    display_agent: Option<String>,
    custom_status: Option<String>,
    state_labels: HashMap<String, String>,
});
