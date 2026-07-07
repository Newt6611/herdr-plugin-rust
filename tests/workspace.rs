use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use herdr_plugin::{HerdrClient, HerdrError, WorkspaceCreateOptions};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);

fn fake_herdr(script: &str) -> PathBuf {
    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "herdr-workspace-client-test-{}-{id}",
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
async fn list_parses_real_workspace_list_shape() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "workspace list" ]; then
  printf '%s\n' '{"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[{"active_tab_id":"wT:t2","agent_status":"working","focused":true,"label":"herdr-plugin-rs","number":4,"pane_count":3,"tab_count":3,"workspace_id":"wT"}]}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let list = client.workspace().list().await.unwrap();

    assert_eq!(list.response_type, "workspace_list");
    assert_eq!(list.workspaces.len(), 1);
    let workspace = &list.workspaces[0];
    assert_eq!(workspace.workspace_id, "wT");
    assert_eq!(workspace.active_tab_id.as_deref(), Some("wT:t2"));
    assert_eq!(workspace.agent_status, "working");
    assert!(workspace.focused);
    assert_eq!(workspace.label, "herdr-plugin-rs");
    assert_eq!(workspace.number, 4);
    assert_eq!(workspace.pane_count, 3);
    assert_eq!(workspace.tab_count, 3);
}

#[tokio::test]
async fn create_sends_supported_options_and_parses_response() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "workspace create --cwd /tmp --label probe --env KEY=VALUE --no-focus" ]; then
  printf '%s\n' '{"id":"cli:workspace:create","result":{"root_pane":{"agent_status":"unknown","cwd":"/private/tmp","focused":false,"foreground_cwd":"/private/tmp","pane_id":"wW:p1","revision":0,"tab_id":"wW:t1","terminal_id":"term_1","workspace_id":"wW"},"tab":{"agent_status":"unknown","focused":false,"label":"1","number":1,"pane_count":1,"tab_id":"wW:t1","workspace_id":"wW"},"type":"workspace_created","workspace":{"active_tab_id":"wW:t1","agent_status":"unknown","focused":false,"label":"probe","number":5,"pane_count":1,"tab_count":1,"workspace_id":"wW"}}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);
    let options = WorkspaceCreateOptions {
        cwd: Some(PathBuf::from("/tmp")),
        label: Some("probe".to_owned()),
        env: vec![("KEY".to_owned(), "VALUE".to_owned())],
        focus: Some(false),
    };

    let created = client.workspace().create(options).await.unwrap();

    assert_eq!(created.response_type, "workspace_created");
    assert_eq!(created.workspace.workspace_id, "wW");
    assert_eq!(created.tab.tab_id, "wW:t1");
    assert_eq!(created.root_pane.cwd, Path::new("/private/tmp"));
}

#[tokio::test]
async fn get_focus_rename_and_close_parse_real_response_shapes() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "workspace get wW")
    printf '%s\n' '{"id":"cli:workspace:get","result":{"type":"workspace_info","workspace":{"active_tab_id":"wW:t1","agent_status":"unknown","focused":false,"label":"probe","number":5,"pane_count":1,"tab_count":1,"workspace_id":"wW"}}}'
    exit 0
    ;;
  "workspace focus wW")
    printf '%s\n' '{"id":"cli:workspace:focus","result":{"type":"workspace_info","workspace":{"active_tab_id":"wW:t1","agent_status":"unknown","focused":true,"label":"probe","number":5,"pane_count":1,"tab_count":1,"workspace_id":"wW"}}}'
    exit 0
    ;;
  "workspace rename wW renamed")
    printf '%s\n' '{"id":"cli:workspace:rename","result":{"type":"workspace_info","workspace":{"active_tab_id":"wW:t1","agent_status":"unknown","focused":true,"label":"renamed","number":5,"pane_count":1,"tab_count":1,"workspace_id":"wW"}}}'
    exit 0
    ;;
  "workspace close wW")
    printf '%s\n' '{"id":"cli:workspace:close","result":{"type":"ok"}}'
    exit 0
    ;;
esac
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let got = client.workspace().get("wW").await.unwrap();
    let focused = client.workspace().focus("wW").await.unwrap();
    let renamed = client.workspace().rename("wW", "renamed").await.unwrap();
    let closed = client.workspace().close("wW").await.unwrap();

    assert_eq!(got.workspace.label, "probe");
    assert!(focused.workspace.focused);
    assert_eq!(renamed.workspace.label, "renamed");
    assert_eq!(closed.response_type, "ok");
}

#[tokio::test]
async fn workspace_json_errors_are_typed() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "workspace get missing" ]; then
  printf '%s\n' '{"error":{"code":"workspace_not_found","message":"workspace missing not found"},"id":"cli:workspace:get"}'
  exit 1
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let error = client.workspace().get("missing").await.unwrap_err();

    match error {
        HerdrError::CommandFailed { error, .. } => {
            assert_eq!(error.code, "workspace_not_found");
            assert_eq!(error.message, "workspace missing not found");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
