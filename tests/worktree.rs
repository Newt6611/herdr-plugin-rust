use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use herdr_plugin::{
    HerdrClient, HerdrError, WorktreeCreateOptions, WorktreeListOptions, WorktreeOpenOptions,
    WorktreeOpenTarget, WorktreeSource,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);

fn fake_herdr(script: &str) -> PathBuf {
    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "herdr-worktree-client-test-{}-{id}",
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
async fn list_parses_real_worktree_list_shape() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "worktree list --cwd /Users/newt/dev/herdr --json" ]; then
  printf '%s\n' '{"id":"cli:worktree:list","result":{"source":{"repo_key":"/Users/newt/dev/herdr/.git","repo_name":"herdr","repo_root":"/Users/newt/dev/herdr","source_checkout_path":"/Users/newt/dev/herdr","source_workspace_id":"wX"},"type":"worktree_list","worktrees":[{"branch":"master","is_bare":false,"is_detached":false,"is_linked_worktree":false,"is_prunable":false,"label":"herdr","open_workspace_id":"wX","path":"/Users/newt/dev/herdr"}]}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let list = client
        .worktree()
        .list(WorktreeListOptions {
            source: Some(WorktreeSource::Cwd(PathBuf::from("/Users/newt/dev/herdr"))),
        })
        .await
        .unwrap();

    assert_eq!(list.response_type, "worktree_list");
    assert_eq!(list.source.repo_name, "herdr");
    assert_eq!(list.source.source_workspace_id.as_deref(), Some("wX"));
    assert_eq!(list.worktrees.len(), 1);
    let worktree = &list.worktrees[0];
    assert_eq!(worktree.branch.as_deref(), Some("master"));
    assert_eq!(worktree.label, "herdr");
    assert_eq!(worktree.open_workspace_id.as_deref(), Some("wX"));
    assert_eq!(worktree.path, Path::new("/Users/newt/dev/herdr"));
}

#[tokio::test]
async fn create_sends_supported_options_and_parses_response() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "worktree create --cwd /Users/newt/dev/herdr --branch probe --base master --path /tmp/probe --label sdk-probe --no-focus --json" ]; then
  printf '%s\n' '{"id":"cli:worktree:create","result":{"root_pane":{"agent_status":"unknown","cwd":"/private/tmp/probe","focused":false,"foreground_cwd":"/private/tmp/probe","pane_id":"wY:p1","revision":0,"tab_id":"wY:t1","terminal_id":"term_1","workspace_id":"wY"},"tab":{"agent_status":"unknown","focused":false,"label":"1","number":1,"pane_count":1,"tab_id":"wY:t1","workspace_id":"wY"},"type":"worktree_created","workspace":{"active_tab_id":"wY:t1","agent_status":"unknown","focused":false,"label":"sdk-probe","number":5,"pane_count":1,"tab_count":1,"workspace_id":"wY","worktree":{"checkout_path":"/tmp/probe","is_linked_worktree":true,"repo_key":"/Users/newt/dev/herdr/.git","repo_name":"herdr","repo_root":"/Users/newt/dev/herdr"}},"worktree":{"branch":"probe","is_bare":false,"is_detached":false,"is_linked_worktree":true,"is_prunable":false,"label":"herdr","open_workspace_id":"wY","path":"/tmp/probe"}}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let created = client
        .worktree()
        .create(WorktreeCreateOptions {
            source: Some(WorktreeSource::Cwd(PathBuf::from("/Users/newt/dev/herdr"))),
            branch: Some("probe".to_owned()),
            base: Some("master".to_owned()),
            path: Some(PathBuf::from("/tmp/probe")),
            label: Some("sdk-probe".to_owned()),
            focus: Some(false),
        })
        .await
        .unwrap();

    assert_eq!(created.response_type, "worktree_created");
    assert_eq!(created.workspace.workspace_id, "wY");
    assert_eq!(created.worktree.branch.as_deref(), Some("probe"));
    assert_eq!(created.worktree.open_workspace_id.as_deref(), Some("wY"));
    assert_eq!(created.root_pane.cwd, Path::new("/private/tmp/probe"));
}

#[tokio::test]
async fn open_sends_target_options_and_parses_response() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "worktree open --workspace wX --branch probe --label sdk-probe-open --focus --json" ]; then
  printf '%s\n' '{"id":"cli:worktree:open","result":{"already_open":true,"root_pane":{"agent_status":"unknown","cwd":"/private/tmp/probe","focused":true,"foreground_cwd":"/private/tmp/probe","pane_id":"wY:p1","revision":0,"tab_id":"wY:t1","terminal_id":"term_1","workspace_id":"wY"},"tab":{"agent_status":"unknown","focused":true,"label":"1. probe","number":1,"pane_count":1,"tab_id":"wY:t1","workspace_id":"wY"},"type":"worktree_opened","workspace":{"active_tab_id":"wY:t1","agent_status":"unknown","focused":true,"label":"sdk-probe-open","number":5,"pane_count":1,"tab_count":1,"workspace_id":"wY","worktree":{"checkout_path":"/private/tmp/probe","is_linked_worktree":true,"repo_key":"/Users/newt/dev/herdr/.git","repo_name":"herdr","repo_root":"/Users/newt/dev/herdr"}},"worktree":{"branch":"probe","is_bare":false,"is_detached":false,"is_linked_worktree":true,"is_prunable":false,"label":"herdr","open_workspace_id":"wY","path":"/private/tmp/probe"}}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let opened = client
        .worktree()
        .open(WorktreeOpenOptions {
            source: Some(WorktreeSource::Workspace("wX".to_owned())),
            target: WorktreeOpenTarget::Branch("probe".to_owned()),
            label: Some("sdk-probe-open".to_owned()),
            focus: Some(true),
        })
        .await
        .unwrap();

    assert_eq!(opened.response_type, "worktree_opened");
    assert!(opened.already_open);
    assert_eq!(opened.workspace.label, "sdk-probe-open");
    assert_eq!(opened.worktree.path, Path::new("/private/tmp/probe"));
}

#[tokio::test]
async fn remove_sends_workspace_and_force_and_parses_response() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "worktree remove --workspace wY --force --json" ]; then
  printf '%s\n' '{"id":"cli:worktree:remove","result":{"forced":true,"path":"/private/tmp/probe","type":"worktree_removed","workspace_id":"wY"}}'
  exit 0
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let removed = client.worktree().remove("wY", true).await.unwrap();

    assert_eq!(removed.response_type, "worktree_removed");
    assert!(removed.forced);
    assert_eq!(removed.workspace_id, "wY");
    assert_eq!(removed.path, Path::new("/private/tmp/probe"));
}

#[tokio::test]
async fn worktree_json_errors_are_typed() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "worktree remove --workspace missing --json" ]; then
  printf '%s\n' '{"error":{"code":"workspace_not_found","message":"workspace missing not found"},"id":"cli:worktree:remove"}'
  exit 1
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let error = client
        .worktree()
        .remove("missing", false)
        .await
        .unwrap_err();

    match error {
        HerdrError::CommandFailed { error, .. } => {
            assert_eq!(error.code, "workspace_not_found");
            assert_eq!(error.message, "workspace missing not found");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
