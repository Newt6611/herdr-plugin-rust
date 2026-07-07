use herdr_plugin::{
    events::{
        AgentStatus, EventData, EventEnvelope, EventKind, PaneAgentStatusChanged, PaneInfo,
        TabInfo, TabRenamed, WorkspaceInfo,
    },
    App, Context,
};

async fn tab_renamed(_ctx: Context, _event: TabRenamed) {}

#[test]
fn workspace_tab_and_pane_event_names_match_herdr_schema() {
    assert_eq!(EventKind::WorkspaceCreated.dot_name(), "workspace.created");
    assert_eq!(EventKind::WorkspaceRenamed.dot_name(), "workspace.renamed");
    assert_eq!(EventKind::TabCreated.dot_name(), "tab.created");
    assert_eq!(EventKind::TabRenamed.dot_name(), "tab.renamed");
    assert_eq!(EventKind::PaneCreated.dot_name(), "pane.created");
    assert_eq!(
        EventKind::PaneAgentStatusChanged.dot_name(),
        "pane.agent_status_changed"
    );
}

#[test]
fn event_envelope_deserializes_tab_renamed_payload_from_herdr_shape() {
    let event: EventEnvelope = serde_json::from_str(
        r#"{
          "event": "tab_renamed",
          "data": {
            "type": "tab_renamed",
            "tab_id": "wT:t2",
            "workspace_id": "wT",
            "label": "codex"
          }
        }"#,
    )
    .unwrap();

    assert_eq!(event.event, EventKind::TabRenamed);
    assert_eq!(
        event.data,
        EventData::TabRenamed {
            tab_id: "wT:t2".to_owned(),
            workspace_id: "wT".to_owned(),
            label: "codex".to_owned(),
        }
    );
}

#[test]
fn event_payload_structs_match_workspace_tab_and_pane_info_shapes() {
    let workspace: WorkspaceInfo = serde_json::from_str(
        r#"{
          "workspace_id":"wT",
          "number":1,
          "label":"herdr-plugin-rs",
          "focused":true,
          "pane_count":3,
          "tab_count":2,
          "active_tab_id":"wT:t2",
          "agent_status":"working"
        }"#,
    )
    .unwrap();
    let tab: TabInfo = serde_json::from_str(
        r#"{
          "tab_id":"wT:t2",
          "workspace_id":"wT",
          "number":2,
          "label":"codex",
          "focused":true,
          "pane_count":1,
          "agent_status":"working"
        }"#,
    )
    .unwrap();
    let pane: PaneInfo = serde_json::from_str(
        r#"{
          "pane_id":"wT:p2",
          "terminal_id":"term_1",
          "workspace_id":"wT",
          "tab_id":"wT:t2",
          "focused":true,
          "cwd":"/repo",
          "foreground_cwd":"/repo",
          "agent":"codex",
          "agent_status":"working",
          "revision":7
        }"#,
    )
    .unwrap();
    let status = PaneAgentStatusChanged {
        pane_id: pane.pane_id.clone(),
        workspace_id: workspace.workspace_id.clone(),
        agent_status: AgentStatus::Working,
        agent: pane.agent.clone(),
        title: None,
        display_agent: None,
        custom_status: None,
        state_labels: Default::default(),
    };

    assert_eq!(workspace.active_tab_id, tab.tab_id);
    assert_eq!(status.pane_id, "wT:p2");
}

#[tokio::test]
async fn app_can_register_sdk_event_types() {
    let mut app = App::new();

    app.on::<TabRenamed>(tab_renamed);
}
