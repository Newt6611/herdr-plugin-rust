use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::events::{AgentStatus, EventEnvelope, WorkspaceWorktreeInfo};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HerdrEnv {
    pub is_herdr: bool,
    pub socket_path: Option<PathBuf>,
    pub bin_path: Option<PathBuf>,
    pub plugin_id: Option<String>,
    pub plugin_root: Option<PathBuf>,
    pub plugin_config_dir: Option<PathBuf>,
    pub plugin_state_dir: Option<PathBuf>,
    pub plugin_context_json: Option<String>,
    pub plugin_context: Option<PluginInvocationContext>,
    pub workspace_id: Option<String>,
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
    pub plugin_action_id: Option<String>,
    pub plugin_event: Option<String>,
    pub plugin_event_json_raw: Option<String>,
    pub plugin_event_json: Option<EventEnvelope>,
    pub plugin_entrypoint_id: Option<String>,
    pub plugin_clicked_url: Option<String>,
    pub plugin_link_handler_id: Option<String>,
}

impl HerdrEnv {
    pub fn from_env() -> Self {
        let plugin_context_json = string_var("HERDR_PLUGIN_CONTEXT_JSON");
        let plugin_context = plugin_context_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok());
        let plugin_event_json_raw = string_var("HERDR_PLUGIN_EVENT_JSON");
        let plugin_event_json = plugin_event_json_raw
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok());

        Self {
            is_herdr: string_var("HERDR_ENV").as_deref() == Some("1"),
            socket_path: path_var("HERDR_SOCKET_PATH"),
            bin_path: path_var("HERDR_BIN_PATH"),
            plugin_id: string_var("HERDR_PLUGIN_ID"),
            plugin_root: path_var("HERDR_PLUGIN_ROOT"),
            plugin_config_dir: path_var("HERDR_PLUGIN_CONFIG_DIR"),
            plugin_state_dir: path_var("HERDR_PLUGIN_STATE_DIR"),
            plugin_context_json,
            plugin_context,
            workspace_id: string_var("HERDR_WORKSPACE_ID"),
            tab_id: string_var("HERDR_TAB_ID"),
            pane_id: string_var("HERDR_PANE_ID"),
            plugin_action_id: string_var("HERDR_PLUGIN_ACTION_ID"),
            plugin_event: string_var("HERDR_PLUGIN_EVENT"),
            plugin_event_json_raw,
            plugin_event_json,
            plugin_entrypoint_id: string_var("HERDR_PLUGIN_ENTRYPOINT_ID"),
            plugin_clicked_url: string_var("HERDR_PLUGIN_CLICKED_URL"),
            plugin_link_handler_id: string_var("HERDR_PLUGIN_LINK_HANDLER_ID"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInvocationContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorkspaceWorktreeInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_status: Option<AgentStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clicked_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_handler_id: Option<String>,
}

fn string_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn path_var(name: &str) -> Option<PathBuf> {
    string_var(name).map(PathBuf::from)
}
