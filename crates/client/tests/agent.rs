use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use herdr_client::{
    AgentExplainOptions, AgentReadOptions, AgentReadSource, AgentStartOptions, AgentWaitStatus,
    Direction, HerdrClient,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);

fn fake_herdr(script: &str) -> PathBuf {
    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "herdr-agent-client-test-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let path = dir.join("herdr");
    fs::write(&path, script).unwrap();

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }

    path
}

fn script(body: &str) -> String {
    format!("#!/bin/sh\nset -eu\n{body}\n")
}

#[tokio::test]
async fn agent_list_get_read_and_explain_parse_real_shapes() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "agent list")
    printf '%s\n' '{"id":"cli:agent:list","result":{"agents":[{"agent":"codex","agent_status":"working","cwd":"/repo","focused":true,"foreground_cwd":"/repo","pane_id":"wT:p2","revision":0,"tab_id":"wT:t2","terminal_id":"term_1","workspace_id":"wT"}],"type":"agent_list"}}'
    ;;
  "agent get wT:p2")
    printf '%s\n' '{"id":"cli:agent:get","result":{"agent":{"agent":"codex","agent_status":"working","cwd":"/repo","focused":true,"foreground_cwd":"/repo","pane_id":"wT:p2","revision":0,"tab_id":"wT:t2","terminal_id":"term_1","workspace_id":"wT"},"type":"agent_info"}}'
    ;;
  "agent read wT:p2 --source recent --lines 2 --format text")
    printf '%s\n' '{"id":"cli:agent:read","result":{"read":{"format":"text","pane_id":"wT:p2","revision":0,"source":"recent","tab_id":"wT:t2","text":"hello","truncated":false,"workspace_id":"wT"},"type":"pane_read"}}'
    ;;
  "agent explain wT:p2 --json")
    printf '%s\n' '{"agent":"codex","state":"working","visible_working":true}'
    ;;
  "agent explain --file /tmp/out.txt --agent codex --json")
    printf '%s\n' '{"agent":"codex","state":"idle","visible_idle":true}'
    ;;
  *) exit 99 ;;
esac"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    assert_eq!(
        client.agent().list().await.unwrap().agents[0]
            .agent
            .as_deref(),
        Some("codex")
    );
    assert_eq!(
        client.agent().get("wT:p2").await.unwrap().agent.pane_id,
        "wT:p2"
    );
    assert_eq!(
        client
            .agent()
            .read(
                "wT:p2",
                AgentReadOptions {
                    source: Some(AgentReadSource::Recent),
                    lines: Some(2),
                    format: Some(herdr_client::ReadFormat::Text),
                    ansi: false,
                },
            )
            .await
            .unwrap()
            .read
            .text,
        "hello"
    );
    assert_eq!(
        client
            .agent()
            .explain(AgentExplainOptions::Target {
                target: "wT:p2".to_owned(),
                json: true,
                verbose: false,
            })
            .await
            .unwrap()["state"],
        "working"
    );
    assert_eq!(
        client
            .agent()
            .explain(AgentExplainOptions::File {
                path: PathBuf::from("/tmp/out.txt"),
                agent: "codex".to_owned(),
                json: true,
                verbose: false,
            })
            .await
            .unwrap()["state"],
        "idle"
    );
}

#[tokio::test]
async fn agent_mutating_commands_use_expected_arguments() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "agent send wT:p2 hello")
    printf '%s\n' '{"id":"cli:agent:send","result":{"type":"ok"}}'
    ;;
  "agent rename wT:p2 neo")
    printf '%s\n' '{"id":"cli:agent:rename","result":{"agent":{"agent":"neo","agent_status":"idle","cwd":"/repo","focused":false,"foreground_cwd":"/repo","pane_id":"wT:p2","revision":0,"tab_id":"wT:t2","terminal_id":"term_1","workspace_id":"wT"},"type":"agent_info"}}'
    ;;
  "agent rename wT:p2 --clear")
    printf '%s\n' '{"id":"cli:agent:rename","result":{"agent":{"agent_status":"idle","cwd":"/repo","focused":false,"foreground_cwd":"/repo","pane_id":"wT:p2","revision":0,"tab_id":"wT:t2","terminal_id":"term_1","workspace_id":"wT"},"type":"agent_info"}}'
    ;;
  "agent focus wT:p2")
    printf '%s\n' '{"id":"cli:agent:focus","result":{"agent":{"agent":"codex","agent_status":"idle","cwd":"/repo","focused":true,"foreground_cwd":"/repo","pane_id":"wT:p2","revision":0,"tab_id":"wT:t2","terminal_id":"term_1","workspace_id":"wT"},"type":"agent_info"}}'
    ;;
  "agent wait wT:p2 --status idle --timeout 100")
    exit 0
    ;;
  "agent attach wT:p2 --takeover")
    exit 0
    ;;
  "agent start codex --cwd /repo --workspace wT --tab wT:t2 --split right --env KEY=VALUE --no-focus -- echo hi")
    printf '%s\n' '{"id":"cli:agent:start","result":{"agent":{"agent":"codex","agent_status":"unknown","cwd":"/repo","focused":false,"foreground_cwd":"/repo","pane_id":"wT:p9","revision":0,"tab_id":"wT:t2","terminal_id":"term_9","workspace_id":"wT"},"type":"agent_info"}}'
    ;;
  *) exit 99 ;;
esac"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    client.agent().send("wT:p2", "hello").await.unwrap();
    assert_eq!(
        client
            .agent()
            .rename("wT:p2", Some("neo"))
            .await
            .unwrap()
            .agent
            .agent
            .as_deref(),
        Some("neo")
    );
    client.agent().rename("wT:p2", None).await.unwrap();
    assert!(client.agent().focus("wT:p2").await.unwrap().agent.focused);
    client
        .agent()
        .wait("wT:p2", AgentWaitStatus::Idle, Some(100))
        .await
        .unwrap();
    client.agent().attach("wT:p2", true).await.unwrap();
    let started = client
        .agent()
        .start(AgentStartOptions {
            name: "codex".to_owned(),
            cwd: Some(PathBuf::from("/repo")),
            workspace_id: Some("wT".to_owned()),
            tab_id: Some("wT:t2".to_owned()),
            split: Some(Direction::Right),
            env: vec![("KEY".to_owned(), "VALUE".to_owned())],
            focus: Some(false),
            argv: vec!["echo".to_owned(), "hi".to_owned()],
        })
        .await
        .unwrap();
    assert_eq!(started.agent.pane_id, "wT:p9");
}
