use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use herdr_client::{
    Direction, HerdrClient, HerdrError, PaneListOptions, PaneMoveDestination, PaneMoveOptions,
    PaneSelector, PaneSplitOptions,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);

fn fake_herdr(script: &str) -> PathBuf {
    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "herdr-pane-client-test-{}-{id}",
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
async fn pane_read_methods_parse_real_shapes() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "pane list --workspace wT")
    printf '%s\n' '{"id":"cli:pane:list","result":{"panes":[{"agent_status":"unknown","cwd":"/repo","focused":false,"foreground_cwd":"/repo","pane_id":"wT:p1","revision":0,"tab_id":"wT:t1","terminal_id":"term_1","workspace_id":"wT"}],"type":"pane_list"}}'
    ;;
  "pane current --current")
    printf '%s\n' '{"id":"cli:pane:current","result":{"pane":{"agent":"codex","agent_status":"working","cwd":"/repo","focused":true,"foreground_cwd":"/repo","pane_id":"wT:p2","revision":0,"tab_id":"wT:t2","terminal_id":"term_2","workspace_id":"wT"},"type":"pane_current"}}'
    ;;
  "pane get wT:p1")
    printf '%s\n' '{"id":"cli:pane:get","result":{"pane":{"agent_status":"unknown","cwd":"/repo","focused":false,"foreground_cwd":"/repo","pane_id":"wT:p1","revision":0,"tab_id":"wT:t1","terminal_id":"term_1","workspace_id":"wT"},"type":"pane_info"}}'
    ;;
  "pane layout --pane wT:p1")
    printf '%s\n' '{"id":"cli:pane:layout","result":{"layout":{"area":{"height":40,"width":133,"x":26,"y":1},"focused_pane_id":"wT:p1","panes":[{"focused":true,"pane_id":"wT:p1","rect":{"height":40,"width":133,"x":26,"y":1}}],"splits":[],"tab_id":"wT:t1","workspace_id":"wT","zoomed":false},"type":"pane_layout"}}'
    ;;
  "pane process-info --pane wT:p1")
    printf '%s\n' '{"id":"cli:pane:process_info","result":{"process_info":{"foreground_process_group_id":90129,"foreground_processes":[{"argv":["nvim","."],"argv0":"nvim","cmdline":"nvim .","cwd":"/repo","name":"nvim","pid":90129}],"pane_id":"wT:p1","shell_pid":82236},"type":"pane_process_info"}}'
    ;;
  *) exit 99 ;;
esac"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let list = client
        .pane()
        .list(PaneListOptions {
            workspace_id: Some("wT".to_owned()),
        })
        .await
        .unwrap();
    let current = client.pane().current(PaneSelector::Current).await.unwrap();
    let got = client.pane().get("wT:p1").await.unwrap();
    let layout = client
        .pane()
        .layout(PaneSelector::Pane("wT:p1".to_owned()))
        .await
        .unwrap();
    let process = client
        .pane()
        .process_info(PaneSelector::Pane("wT:p1".to_owned()))
        .await
        .unwrap();

    assert_eq!(list.panes[0].pane_id, "wT:p1");
    assert_eq!(current.pane.agent.as_deref(), Some("codex"));
    assert_eq!(got.pane.cwd, Path::new("/repo"));
    assert_eq!(layout.layout.area.width, 133);
    assert_eq!(process.process_info.foreground_processes[0].name, "nvim");
}

#[tokio::test]
async fn pane_directional_and_mutating_methods_use_expected_arguments() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "pane neighbor --direction right --pane wT:p1")
    printf '%s\n' '{"id":"cli:pane:neighbor","result":{"neighbor":{"direction":"right","pane_id":"wT:p1"},"type":"pane_neighbor"}}'
    ;;
  "pane edges --pane wT:p1")
    printf '%s\n' '{"id":"cli:pane:edges","result":{"edges":{"down":true,"left":true,"pane_id":"wT:p1","right":false,"up":true},"type":"pane_edges"}}'
    ;;
  "pane focus --direction left --pane wT:p1")
    printf '%s\n' '{"id":"cli:pane:focus","result":{"focus":{"changed":true,"focused_pane_id":"wT:p2","source_pane_id":"wT:p1"},"type":"pane_focus_direction"}}'
    ;;
  "pane resize --direction right --amount 0.25 --pane wT:p1")
    printf '%s\n' '{"id":"cli:pane:resize","result":{"resize":{"changed":true,"pane_id":"wT:p1"},"type":"pane_resize"}}'
    ;;
  "pane zoom wT:p1 --toggle")
    printf '%s\n' '{"id":"cli:pane:zoom","result":{"type":"pane_zoom","zoom":{"changed":true,"pane_id":"wT:p1","zoomed":true}}}'
    ;;
  "pane rename wT:p1 probe")
    printf '%s\n' '{"id":"cli:pane:rename","result":{"pane":{"agent_status":"unknown","cwd":"/repo","focused":false,"foreground_cwd":"/repo","label":"probe","pane_id":"wT:p1","revision":0,"tab_id":"wT:t1","terminal_id":"term_1","workspace_id":"wT"},"type":"pane_info"}}'
    ;;
  "pane rename wT:p1 --clear")
    printf '%s\n' '{"id":"cli:pane:rename","result":{"pane":{"agent_status":"unknown","cwd":"/repo","focused":false,"foreground_cwd":"/repo","pane_id":"wT:p1","revision":0,"tab_id":"wT:t1","terminal_id":"term_1","workspace_id":"wT"},"type":"pane_info"}}'
    ;;
  "pane close wT:p1")
    printf '%s\n' '{"id":"cli:pane:close","result":{"type":"ok"}}'
    ;;
  *) exit 99 ;;
esac"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    assert_eq!(
        client
            .pane()
            .neighbor(Direction::Right, PaneSelector::Pane("wT:p1".to_owned()))
            .await
            .unwrap()
            .response_type,
        "pane_neighbor"
    );
    assert!(
        client
            .pane()
            .edges(PaneSelector::Pane("wT:p1".to_owned()))
            .await
            .unwrap()
            .edges
            .left
    );
    client
        .pane()
        .focus(Direction::Left, PaneSelector::Pane("wT:p1".to_owned()))
        .await
        .unwrap();
    client
        .pane()
        .resize(
            Direction::Right,
            Some(0.25),
            PaneSelector::Pane("wT:p1".to_owned()),
        )
        .await
        .unwrap();
    client
        .pane()
        .zoom(
            PaneSelector::Pane("wT:p1".to_owned()),
            herdr_client::PaneZoomMode::Toggle,
        )
        .await
        .unwrap();
    assert_eq!(
        client
            .pane()
            .rename("wT:p1", Some("probe"))
            .await
            .unwrap()
            .pane
            .label
            .as_deref(),
        Some("probe")
    );
    client.pane().rename("wT:p1", None).await.unwrap();
    client.pane().close("wT:p1").await.unwrap();
}

#[tokio::test]
async fn pane_split_swap_and_move_methods_use_expected_arguments() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "pane split wT:p1 --direction down --ratio 0.5 --cwd /tmp --env KEY=VALUE --no-focus")
    printf '%s\n' '{"id":"cli:pane:split","result":{"pane":{"agent_status":"unknown","cwd":"/tmp","focused":false,"foreground_cwd":"/tmp","pane_id":"wT:p2","revision":0,"tab_id":"wT:t1","terminal_id":"term_2","workspace_id":"wT"},"type":"pane_info"}}'
    ;;
  "pane swap --direction up --pane wT:p2")
    printf '%s\n' '{"id":"cli:pane:swap","result":{"swap":{"changed":true,"source_pane_id":"wT:p2"},"type":"pane_swap"}}'
    ;;
  "pane swap --source-pane wT:p1 --target-pane wT:p2")
    printf '%s\n' '{"id":"cli:pane:swap","result":{"swap":{"changed":true,"source_pane_id":"wT:p1","target_pane_id":"wT:p2"},"type":"pane_swap"}}'
    ;;
  "pane move wT:p2 --tab wT:t1 --split right --target-pane wT:p1 --ratio 0.4 --no-focus")
    printf '%s\n' '{"id":"cli:pane:move","result":{"move_result":{"changed":true,"previous_pane_id":"wT:p2"},"type":"pane_move"}}'
    ;;
  "pane move wT:p2 --new-tab --workspace wT --label moved --focus")
    printf '%s\n' '{"id":"cli:pane:move","result":{"move_result":{"changed":true,"previous_pane_id":"wT:p2"},"type":"pane_move"}}'
    ;;
  "pane move wT:p2 --new-workspace --label ws --tab-label tab --no-focus")
    printf '%s\n' '{"id":"cli:pane:move","result":{"move_result":{"changed":true,"previous_pane_id":"wT:p2"},"type":"pane_move"}}'
    ;;
  *) exit 99 ;;
esac"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    client
        .pane()
        .split(PaneSplitOptions {
            pane: PaneSelector::Pane("wT:p1".to_owned()),
            direction: Direction::Down,
            ratio: Some(0.5),
            cwd: Some(PathBuf::from("/tmp")),
            env: vec![("KEY".to_owned(), "VALUE".to_owned())],
            focus: Some(false),
        })
        .await
        .unwrap();
    client
        .pane()
        .swap_direction(Direction::Up, PaneSelector::Pane("wT:p2".to_owned()))
        .await
        .unwrap();
    client.pane().swap_panes("wT:p1", "wT:p2").await.unwrap();
    client
        .pane()
        .move_pane(PaneMoveOptions {
            pane_id: "wT:p2".to_owned(),
            destination: PaneMoveDestination::ExistingTab {
                tab_id: "wT:t1".to_owned(),
                split: Direction::Right,
                target_pane_id: Some("wT:p1".to_owned()),
                ratio: Some(0.4),
            },
            focus: Some(false),
        })
        .await
        .unwrap();
    client
        .pane()
        .move_pane(PaneMoveOptions {
            pane_id: "wT:p2".to_owned(),
            destination: PaneMoveDestination::NewTab {
                workspace_id: Some("wT".to_owned()),
                label: Some("moved".to_owned()),
            },
            focus: Some(true),
        })
        .await
        .unwrap();
    client
        .pane()
        .move_pane(PaneMoveOptions {
            pane_id: "wT:p2".to_owned(),
            destination: PaneMoveDestination::NewWorkspace {
                label: Some("ws".to_owned()),
                tab_label: Some("tab".to_owned()),
            },
            focus: Some(false),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn pane_json_errors_are_typed() {
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "pane get missing" ]; then
  printf '%s\n' '{"error":{"code":"pane_not_found","message":"pane missing not found"},"id":"cli:pane:get"}'
  exit 1
fi
exit 99"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    let error = client.pane().get("missing").await.unwrap_err();

    match error {
        HerdrError::CommandFailed { error, .. } => {
            assert_eq!(error.code, "pane_not_found");
            assert_eq!(error.message, "pane missing not found");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
