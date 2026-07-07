use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use herdr_plugin::{HerdrClient, HerdrError, TabCreateOptions, TabListOptions};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);

fn fake_herdr(script: &str) -> PathBuf {
    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("herdr-tab-client-test-{}-{id}", std::process::id()));
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
async fn list_parses_real_tab_list_shape() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "tab list --workspace wT" ]; then
  printf '%s\n' '{"id":"cli:tab:list","result":{"tabs":[{"agent_status":"working","focused":true,"label":"2. codex","number":2,"pane_count":1,"tab_id":"wT:t2","workspace_id":"wT"}],"type":"tab_list"}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let list = client
        .tab()
        .list(TabListOptions {
            workspace_id: Some("wT".to_owned()),
        })
        .await
        .unwrap();

    assert_eq!(list.response_type, "tab_list");
    assert_eq!(list.tabs.len(), 1);
    let tab = &list.tabs[0];
    assert_eq!(tab.tab_id, "wT:t2");
    assert_eq!(tab.workspace_id, "wT");
    assert_eq!(tab.agent_status, "working");
    assert!(tab.focused);
    assert_eq!(tab.label, "2. codex");
    assert_eq!(tab.number, 2);
    assert_eq!(tab.pane_count, 1);
}

#[tokio::test]
async fn create_sends_supported_options_and_parses_response() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "tab create --workspace wT --cwd /tmp --label probe --env KEY=VALUE --no-focus" ]; then
  printf '%s\n' '{"id":"cli:tab:create","result":{"root_pane":{"agent_status":"unknown","cwd":"/private/tmp","focused":false,"foreground_cwd":"/private/tmp","pane_id":"wT:p5","revision":0,"tab_id":"wT:t4","terminal_id":"term_1","workspace_id":"wT"},"tab":{"agent_status":"unknown","focused":false,"label":"probe","number":4,"pane_count":1,"tab_id":"wT:t4","workspace_id":"wT"},"type":"tab_created"}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let created = client
        .tab()
        .create(TabCreateOptions {
            workspace_id: Some("wT".to_owned()),
            cwd: Some(PathBuf::from("/tmp")),
            label: Some("probe".to_owned()),
            env: vec![("KEY".to_owned(), "VALUE".to_owned())],
            focus: Some(false),
        })
        .await
        .unwrap();

    assert_eq!(created.response_type, "tab_created");
    assert_eq!(created.tab.tab_id, "wT:t4");
    assert_eq!(created.root_pane.cwd, Path::new("/private/tmp"));
}

#[tokio::test]
async fn get_focus_rename_and_close_parse_real_response_shapes() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "tab get wT:t4")
    printf '%s\n' '{"id":"cli:tab:get","result":{"tab":{"agent_status":"unknown","focused":false,"label":"4. tmp","number":4,"pane_count":1,"tab_id":"wT:t4","workspace_id":"wT"},"type":"tab_info"}}'
    exit 0
    ;;
  "tab focus wT:t4")
    printf '%s\n' '{"id":"cli:tab:focus","result":{"tab":{"agent_status":"unknown","focused":true,"label":"4. tmp","number":4,"pane_count":1,"tab_id":"wT:t4","workspace_id":"wT"},"type":"tab_info"}}'
    exit 0
    ;;
  "tab rename wT:t4 renamed")
    printf '%s\n' '{"id":"cli:tab:rename","result":{"tab":{"agent_status":"unknown","focused":true,"label":"renamed","number":4,"pane_count":1,"tab_id":"wT:t4","workspace_id":"wT"},"type":"tab_info"}}'
    exit 0
    ;;
  "tab close wT:t4")
    printf '%s\n' '{"id":"cli:tab:close","result":{"type":"ok"}}'
    exit 0
    ;;
esac
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let got = client.tab().get("wT:t4").await.unwrap();
    let focused = client.tab().focus("wT:t4").await.unwrap();
    let renamed = client.tab().rename("wT:t4", "renamed").await.unwrap();
    let closed = client.tab().close("wT:t4").await.unwrap();

    assert_eq!(got.tab.label, "4. tmp");
    assert!(focused.tab.focused);
    assert_eq!(renamed.tab.label, "renamed");
    assert_eq!(closed.response_type, "ok");
}

#[tokio::test]
async fn tab_json_errors_are_typed() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "tab get missing" ]; then
  printf '%s\n' '{"error":{"code":"tab_not_found","message":"tab missing not found"},"id":"cli:tab:get"}'
  exit 1
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let error = client.tab().get("missing").await.unwrap_err();

    match error {
        HerdrError::CommandFailed { error, .. } => {
            assert_eq!(error.code, "tab_not_found");
            assert_eq!(error.message, "tab missing not found");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
