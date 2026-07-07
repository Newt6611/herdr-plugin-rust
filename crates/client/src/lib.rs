//! Client primitives for Herdr CLI and socket integration.

mod agent;
mod client;
mod error;
mod models;
mod pane;
mod session;
mod tab;
mod workspace;
mod worktree;

pub use agent::{
    AgentClient, AgentExplainOptions, AgentReadOptions, AgentReadSource, AgentStartOptions,
    AgentWaitStatus, ReadFormat,
};
pub use client::{CommandLine, HerdrClient};
pub use error::{HerdrCommandError, HerdrError};
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
