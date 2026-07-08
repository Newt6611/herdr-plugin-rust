use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SessionList {
    pub sessions: Vec<Session>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Session {
    #[serde(rename = "default")]
    pub is_default: bool,
    pub name: String,
    pub running: bool,
    pub session_dir: PathBuf,
    pub socket_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct StopSessionResponse {
    pub stopped: bool,
    pub session: Session,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DeleteSessionResponse {
    pub deleted: bool,
    pub session: Session,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorkspaceList {
    #[serde(rename = "type")]
    pub response_type: String,
    pub workspaces: Vec<Workspace>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorkspaceInfoResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub workspace: Workspace,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorkspaceCreateResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub workspace: Workspace,
    pub tab: WorkspaceTab,
    pub root_pane: WorkspacePane,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorkspaceCloseResponse {
    #[serde(rename = "type")]
    pub response_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Workspace {
    pub workspace_id: String,
    pub active_tab_id: Option<String>,
    pub agent_status: String,
    pub focused: bool,
    pub label: String,
    pub number: u64,
    pub pane_count: u64,
    pub tab_count: u64,
    pub worktree: Option<WorkspaceWorktree>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorkspaceTab {
    pub workspace_id: String,
    pub tab_id: String,
    pub agent_status: String,
    pub focused: bool,
    pub label: String,
    pub number: u64,
    pub pane_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorkspacePane {
    pub workspace_id: String,
    pub tab_id: String,
    pub pane_id: String,
    pub terminal_id: String,
    pub agent_status: String,
    pub cwd: PathBuf,
    pub foreground_cwd: PathBuf,
    pub focused: bool,
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorkspaceWorktree {
    pub checkout_path: PathBuf,
    pub is_linked_worktree: bool,
    pub repo_key: PathBuf,
    pub repo_name: String,
    pub repo_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorktreeList {
    #[serde(rename = "type")]
    pub response_type: String,
    pub source: WorktreeSourceInfo,
    pub worktrees: Vec<Worktree>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorktreeSourceInfo {
    pub repo_key: PathBuf,
    pub repo_name: String,
    pub repo_root: PathBuf,
    pub source_checkout_path: PathBuf,
    pub source_workspace_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Worktree {
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_detached: bool,
    pub is_linked_worktree: bool,
    pub is_prunable: bool,
    pub label: String,
    pub open_workspace_id: Option<String>,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorktreeCreateResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub workspace: Workspace,
    pub tab: WorkspaceTab,
    pub root_pane: WorkspacePane,
    pub worktree: Worktree,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorktreeOpenResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub already_open: bool,
    pub workspace: Workspace,
    pub tab: WorkspaceTab,
    pub root_pane: WorkspacePane,
    pub worktree: Worktree,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WorktreeRemoveResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub forced: bool,
    pub path: PathBuf,
    pub workspace_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TabList {
    #[serde(rename = "type")]
    pub response_type: String,
    pub tabs: Vec<Tab>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TabInfoResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub tab: Tab,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TabCreateResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub tab: Tab,
    pub root_pane: TabPane,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TabCloseResponse {
    #[serde(rename = "type")]
    pub response_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Tab {
    pub workspace_id: String,
    pub tab_id: String,
    pub agent_status: String,
    pub focused: bool,
    pub label: String,
    pub number: u64,
    pub pane_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TabPane {
    pub workspace_id: String,
    pub tab_id: String,
    pub pane_id: String,
    pub terminal_id: String,
    pub agent_status: String,
    pub cwd: PathBuf,
    pub foreground_cwd: PathBuf,
    pub focused: bool,
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PaneList {
    #[serde(rename = "type")]
    pub response_type: String,
    pub panes: Vec<Pane>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PaneCurrentResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub pane: Pane,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PaneInfoResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub pane: Pane,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Pane {
    pub workspace_id: String,
    pub tab_id: String,
    pub pane_id: String,
    pub terminal_id: String,
    pub agent_status: String,
    pub cwd: PathBuf,
    pub foreground_cwd: PathBuf,
    pub focused: bool,
    pub revision: u64,
    pub agent: Option<String>,
    pub agent_session: Option<AgentSession>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AgentSession {
    pub agent: String,
    pub kind: String,
    pub source: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneLayoutResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub layout: PaneLayout,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneLayout {
    pub area: PaneRect,
    pub focused_pane_id: Option<String>,
    pub panes: Vec<PaneSplitPane>,
    pub splits: Vec<PaneSplit>,
    pub tab_id: String,
    pub workspace_id: String,
    pub zoomed: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneRect {
    pub height: u64,
    pub width: u64,
    pub x: u64,
    pub y: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneSplitPane {
    pub focused: bool,
    pub pane_id: String,
    pub rect: PaneRect,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneSplit {
    pub direction: String,
    pub id: String,
    pub ratio: f64,
    pub rect: PaneRect,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneProcessInfoResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub process_info: ProcessInfo,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProcessInfo {
    pub foreground_process_group_id: Option<u64>,
    pub foreground_processes: Vec<ProcessInfoProcess>,
    pub pane_id: String,
    pub shell_pid: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProcessInfoProcess {
    pub argv: Vec<String>,
    pub argv0: Option<String>,
    pub cmdline: String,
    pub cwd: Option<PathBuf>,
    pub name: String,
    pub pid: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneActionResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(flatten)]
    pub payload: serde_json::Map<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneEdgesResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub edges: PaneEdges,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaneEdges {
    pub pane_id: String,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub layout: Option<PaneLayout>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PaneCloseResponse {
    #[serde(rename = "type")]
    pub response_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PluginPaneInfo {
    pub plugin_id: String,
    pub entrypoint: String,
    pub pane: Pane,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PluginPaneOpenResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub plugin_pane: PluginPaneInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PluginPaneFocusResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub plugin_pane: PluginPaneInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PluginPaneCloseResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub pane_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InstalledPluginInfo {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub min_herdr_version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub manifest_path: String,
    pub plugin_root: String,
    pub enabled: bool,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
    #[serde(default)]
    pub build: Vec<Value>,
    #[serde(default)]
    pub actions: Vec<Value>,
    #[serde(default)]
    pub events: Vec<Value>,
    #[serde(default)]
    pub panes: Vec<Value>,
    #[serde(default)]
    pub link_handlers: Vec<Value>,
    #[serde(default)]
    pub source: PluginSourceInfo,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct PluginSourceInfo {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub subdir: Option<String>,
    #[serde(default)]
    pub requested_ref: Option<String>,
    #[serde(default)]
    pub resolved_commit: Option<String>,
    #[serde(default)]
    pub managed_path: Option<String>,
    #[serde(default)]
    pub installed_unix_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PluginListResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub plugins: Vec<InstalledPluginInfo>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PluginEnableResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub plugin: InstalledPluginInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PluginDisableResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub plugin: InstalledPluginInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AgentList {
    #[serde(rename = "type")]
    pub response_type: String,
    pub agents: Vec<Pane>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AgentInfoResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub agent: Pane,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct AgentReadResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub read: PaneRead,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PaneRead {
    pub format: String,
    pub pane_id: String,
    pub revision: u64,
    pub source: String,
    pub tab_id: Option<String>,
    pub text: String,
    pub truncated: bool,
    pub workspace_id: String,
}
