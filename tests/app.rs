use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex as StdMutex, MutexGuard,
    },
};

use herdr_plugin::HerdrClient;
use herdr_plugin::{
    event_source::{EnvEventSource, RuntimeEvent},
    events::{Event, EventKind, TabRenamed},
    App, Context, EnvRuntime, EventSourceOutput, HerdrEnv, PluginInvocationContext, Runtime,
    RuntimeApp, RuntimeError, RuntimeFuture,
};
use serde::Deserialize;
use tokio::sync::Mutex;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);
static ENV_LOCK: StdMutex<()> = StdMutex::new(());

const HERDR_ENV_KEYS: &[&str] = &[
    "HERDR_SOCKET_PATH",
    "HERDR_BIN_PATH",
    "HERDR_ENV",
    "HERDR_PLUGIN_ID",
    "HERDR_PLUGIN_ROOT",
    "HERDR_PLUGIN_CONFIG_DIR",
    "HERDR_PLUGIN_STATE_DIR",
    "HERDR_PLUGIN_CONTEXT_JSON",
    "HERDR_WORKSPACE_ID",
    "HERDR_TAB_ID",
    "HERDR_PANE_ID",
    "HERDR_PLUGIN_ACTION_ID",
    "HERDR_PLUGIN_EVENT",
    "HERDR_PLUGIN_EVENT_JSON",
    "HERDR_PLUGIN_ENTRYPOINT_ID",
];

#[derive(Debug)]
struct EnvGuard {
    previous: Vec<(&'static str, Option<OsString>)>,
    _guard: MutexGuard<'static, ()>,
}

fn fake_herdr(script: &str) -> PathBuf {
    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("herdr-plugin-test-{}-{id}", std::process::id()));
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

impl EnvGuard {
    fn set(vars: &[(&str, &str)]) -> Self {
        let guard = ENV_LOCK.lock().unwrap();
        let previous = HERDR_ENV_KEYS
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect::<Vec<(&str, Option<OsString>)>>();

        for key in HERDR_ENV_KEYS {
            std::env::remove_var(key);
        }
        for (key, value) in vars {
            std::env::set_var(key, value);
        }

        Self {
            previous,
            _guard: guard,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[tokio::test]
async fn app_builder_runs_event_from_env() {
    let _env = EnvGuard::set(&[(
        "HERDR_PLUGIN_EVENT_JSON",
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"w1:t1",
            "workspace_id":"w1",
            "label":"Renamed"
          }
        }"#,
    )]);
    let seen = Arc::new(Mutex::new(Vec::<u64>::new()));
    let app = App::builder().build().unwrap().on_event::<TabRenamed>({
        let seen = seen.clone();
        move |_ctx: Context, event: TabRenamed| {
            let seen = seen.clone();
            async move {
                seen.lock().await.push(event.label.len() as u64);
            }
        }
    });
    app.run().await.unwrap();

    assert_eq!(*seen.lock().await, [7]);
}

#[tokio::test]
async fn explicit_env_runtime_runs_event_from_env() {
    let _env = EnvGuard::set(&[(
        "HERDR_PLUGIN_EVENT_JSON",
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"w1:t1",
            "workspace_id":"w1",
            "label":"Renamed"
          }
        }"#,
    )]);
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let app = App::builder()
        .runtime(EnvRuntime::new())
        .build()
        .unwrap()
        .on_event::<TabRenamed>({
            let seen = seen.clone();
            move |_ctx: Context, event: TabRenamed| {
                let seen = seen.clone();
                async move {
                    seen.lock().await.push(event.label);
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(*seen.lock().await, ["Renamed"]);
}

struct RecordingRuntime {
    calls: Arc<StdMutex<Vec<&'static str>>>,
}

impl<State, Config> Runtime<State, Config> for RecordingRuntime
where
    State: Send + Sync + 'static,
    Config: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
{
    fn run(self, mut app: RuntimeApp<State, Config>) -> RuntimeFuture {
        Box::pin(async move {
            self.calls.lock().unwrap().push("run");
            app.initialize(EventSourceOutput {
                env: HerdrEnv::default(),
                event: None,
            })?;
            if let Err(source) = app.run_setup().await {
                return app.return_error(RuntimeError::Setup { source }).await;
            }
            if let Err(source) = app.run_teardown().await {
                return app.return_error(RuntimeError::Teardown { source }).await;
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn app_run_delegates_lifecycle_to_configured_runtime() {
    let calls = Arc::new(StdMutex::new(Vec::new()));
    let app = App::builder()
        .runtime(RecordingRuntime {
            calls: calls.clone(),
        })
        .build()
        .unwrap()
        .setup({
            let calls = calls.clone();
            move |_ctx: Context| {
                let calls = calls.clone();
                async move {
                    calls.lock().unwrap().push("setup");
                    Ok(())
                }
            }
        })
        .teardown({
            let calls = calls.clone();
            move |_ctx: Context| {
                let calls = calls.clone();
                async move {
                    calls.lock().unwrap().push("teardown");
                    Ok(())
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(*calls.lock().unwrap(), ["run", "setup", "teardown"]);
}

#[tokio::test]
async fn app_with_client_shares_arc_client_with_handlers() {
    let _env = EnvGuard::set(&[(
        "HERDR_PLUGIN_EVENT_JSON",
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"w1:t1",
            "workspace_id":"w1",
            "label":"Renamed"
          }
        }"#,
    )]);
    let client = Arc::new(HerdrClient::with_binary("/definitely/missing/herdr"));
    let seen_same_client = Arc::new(Mutex::new(false));
    let app = App::builder()
        .with_client(client.clone())
        .build()
        .unwrap()
        .on_event::<TabRenamed>({
            let client = client.clone();
            let seen_same_client = seen_same_client.clone();
            move |ctx: Context, _event: TabRenamed| {
                let client = client.clone();
                let seen_same_client = seen_same_client.clone();
                async move {
                    *seen_same_client.lock().await = std::ptr::eq(ctx.client(), client.as_ref());
                }
            }
        });
    app.run().await.unwrap();

    assert!(*seen_same_client.lock().await);
}

#[tokio::test]
async fn app_builder_sets_herdr_binary_path_for_context_client() {
    let _env = EnvGuard::set(&[(
        "HERDR_PLUGIN_EVENT_JSON",
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"w1:t1",
            "workspace_id":"w1",
            "label":"Renamed"
          }
        }"#,
    )]);
    let herdr = fake_herdr(&script(
        r#"if [ "$*" = "session list --json" ]; then
  printf '%s\n' '{"sessions":[]}'
  exit 0
fi
exit 99"#,
    ));
    let seen_session_count = Arc::new(Mutex::new(None));
    let app = App::builder()
        .with_herdr_bin_path(herdr)
        .build()
        .unwrap()
        .on_event::<TabRenamed>({
            let seen_session_count = seen_session_count.clone();
            move |ctx: Context, _event: TabRenamed| {
                let seen_session_count = seen_session_count.clone();
                async move {
                    let sessions = ctx.client().session().list().await.unwrap();
                    *seen_session_count.lock().await = Some(sessions.sessions.len());
                }
            }
        });
    app.run().await.unwrap();

    assert_eq!(*seen_session_count.lock().await, Some(0));
}

#[tokio::test]
async fn setup_runs_with_context_before_event_dispatch() {
    let _env = EnvGuard::set(&[
        ("HERDR_PLUGIN_ID", "setup-test"),
        (
            "HERDR_PLUGIN_EVENT_JSON",
            r#"{
              "event":"tab_renamed",
              "data":{
                "type":"tab_renamed",
                "tab_id":"w1:t1",
                "workspace_id":"w1",
                "label":"Renamed"
              }
            }"#,
        ),
    ]);
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let app = App::builder()
        .build()
        .unwrap()
        .setup({
            let calls = calls.clone();
            move |ctx: Context| {
                let calls = calls.clone();
                async move {
                    calls
                        .lock()
                        .await
                        .push(format!("setup:{}", ctx.env().plugin_id.as_deref().unwrap()));
                    Ok(())
                }
            }
        })
        .on_event::<TabRenamed>({
            let calls = calls.clone();
            move |_ctx: Context, event: TabRenamed| {
                let calls = calls.clone();
                async move {
                    calls.lock().await.push(format!("event:{}", event.label));
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(*calls.lock().await, ["setup:setup-test", "event:Renamed"]);
}

#[tokio::test]
async fn app_state_is_available_to_setup_and_event_handlers() {
    #[derive(Debug)]
    struct State {
        prefix: String,
    }

    let _env = EnvGuard::set(&[(
        "HERDR_PLUGIN_EVENT_JSON",
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"w1:t1",
            "workspace_id":"w1",
            "label":"Renamed"
          }
        }"#,
    )]);
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let app = App::builder()
        .with_state(State {
            prefix: "state".to_string(),
        })
        .build()
        .unwrap()
        .setup({
            let calls = calls.clone();
            move |ctx: Context<State>| {
                let calls = calls.clone();
                async move {
                    let original_prefix = {
                        let state = ctx.state();
                        state.prefix.clone()
                    };
                    ctx.state_mut().prefix = "initialized".to_string();
                    calls.lock().await.push(format!("setup:{original_prefix}"));
                    Ok(())
                }
            }
        })
        .on_event::<TabRenamed>({
            let calls = calls.clone();
            move |ctx: Context<State>, event: TabRenamed| {
                let calls = calls.clone();
                async move {
                    let prefix = {
                        let state = ctx.state();
                        state.prefix.clone()
                    };
                    calls
                        .lock()
                        .await
                        .push(format!("event:{prefix}:{}", event.label));
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(
        *calls.lock().await,
        ["setup:state", "event:initialized:Renamed"]
    );
}

#[tokio::test]
async fn context_exposes_invocation_helpers_paths_and_logger() {
    let _env = EnvGuard::set(&[
        ("HERDR_PLUGIN_CONFIG_DIR", "/tmp/herdr-config"),
        ("HERDR_PLUGIN_STATE_DIR", "/tmp/herdr-state"),
        ("HERDR_PLUGIN_ACTION_ID", "refresh"),
        (
            "HERDR_PLUGIN_EVENT_JSON",
            r#"{
              "event":"tab_renamed",
              "data":{
                "type":"tab_renamed",
                "tab_id":"w1:t1",
                "workspace_id":"w1",
                "label":"Renamed"
              }
            }"#,
        ),
    ]);
    let captured = Arc::new(Mutex::new(false));
    let app = App::builder().build().unwrap().setup({
        let captured = captured.clone();
        move |ctx: Context| {
            let captured = captured.clone();
            async move {
                ctx.log().info("setup");
                assert!(ctx.is_event());
                assert!(ctx.is_action());
                assert_eq!(ctx.event_kind(), Some(EventKind::TabRenamed));
                assert_eq!(
                    ctx.config_path("config.toml").as_deref(),
                    Some(Path::new("/tmp/herdr-config/config.toml"))
                );
                assert_eq!(
                    ctx.state_path("state.json").as_deref(),
                    Some(Path::new("/tmp/herdr-state/state.json"))
                );
                *captured.lock().await = true;
                Ok(())
            }
        }
    });

    app.run().await.unwrap();

    assert!(*captured.lock().await);
}

#[derive(Debug, Default, Deserialize)]
struct TestConfig {
    label_prefix: String,
    debounce_ms: u64,
}

#[tokio::test]
async fn config_loads_default_toml_before_setup_and_event_dispatch() {
    let config_dir = std::env::temp_dir().join(format!(
        "herdr-plugin-config-{}-{}",
        std::process::id(),
        NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        r#"
label_prefix = "cfg"
debounce_ms = 250
"#,
    )
    .unwrap();

    let _env = EnvGuard::set(&[
        ("HERDR_PLUGIN_CONFIG_DIR", config_dir.to_str().unwrap()),
        (
            "HERDR_PLUGIN_EVENT_JSON",
            r#"{
              "event":"tab_renamed",
              "data":{
                "type":"tab_renamed",
                "tab_id":"w1:t1",
                "workspace_id":"w1",
                "label":"Renamed"
              }
            }"#,
        ),
    ]);
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let app = App::builder()
        .with_config::<TestConfig>()
        .build()
        .unwrap()
        .setup({
            let calls = calls.clone();
            move |ctx: Context<(), TestConfig>| {
                let calls = calls.clone();
                async move {
                    calls
                        .lock()
                        .await
                        .push(format!("setup:{}", ctx.config().label_prefix));
                    Ok(())
                }
            }
        })
        .on_event::<TabRenamed>({
            let calls = calls.clone();
            move |ctx: Context<(), TestConfig>, event: TabRenamed| {
                let calls = calls.clone();
                async move {
                    calls.lock().await.push(format!(
                        "event:{}:{}:{}",
                        ctx.config().label_prefix,
                        ctx.config().debounce_ms,
                        event.label
                    ));
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(*calls.lock().await, ["setup:cfg", "event:cfg:250:Renamed"]);
}

#[tokio::test]
async fn config_can_load_custom_relative_toml_file() {
    let config_dir = std::env::temp_dir().join(format!(
        "herdr-plugin-config-{}-{}",
        std::process::id(),
        NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("settings.toml"),
        r#"
label_prefix = "settings"
debounce_ms = 500
"#,
    )
    .unwrap();

    let _env = EnvGuard::set(&[("HERDR_PLUGIN_CONFIG_DIR", config_dir.to_str().unwrap())]);
    let captured = Arc::new(Mutex::new(None));
    let app = App::builder()
        .with_config_file::<TestConfig>("settings.toml")
        .build()
        .unwrap()
        .setup({
            let captured = captured.clone();
            move |ctx: Context<(), TestConfig>| {
                let captured = captured.clone();
                async move {
                    *captured.lock().await =
                        Some((ctx.config().label_prefix.clone(), ctx.config().debounce_ms));
                    Ok(())
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(*captured.lock().await, Some(("settings".to_string(), 500)));
}

#[tokio::test]
async fn missing_config_file_uses_default_config() {
    let config_dir = std::env::temp_dir().join(format!(
        "herdr-plugin-config-{}-{}",
        std::process::id(),
        NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&config_dir).unwrap();

    let _env = EnvGuard::set(&[("HERDR_PLUGIN_CONFIG_DIR", config_dir.to_str().unwrap())]);
    let captured = Arc::new(Mutex::new(None));
    let app = App::builder()
        .with_config::<TestConfig>()
        .build()
        .unwrap()
        .setup({
            let captured = captured.clone();
            move |ctx: Context<(), TestConfig>| {
                let captured = captured.clone();
                async move {
                    *captured.lock().await =
                        Some((ctx.config().label_prefix.clone(), ctx.config().debounce_ms));
                    Ok(())
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(*captured.lock().await, Some(("".to_string(), 0)));
}

#[tokio::test]
async fn invalid_config_toml_returns_typed_runtime_error() {
    let config_dir = std::env::temp_dir().join(format!(
        "herdr-plugin-config-{}-{}",
        std::process::id(),
        NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), "label_prefix = [").unwrap();

    let _env = EnvGuard::set(&[("HERDR_PLUGIN_CONFIG_DIR", config_dir.to_str().unwrap())]);
    let app = App::builder().with_config::<TestConfig>().build().unwrap();
    let error = app.run().await.unwrap_err();

    assert!(matches!(error, RuntimeError::Config { .. }));
}

#[tokio::test]
async fn teardown_runs_after_event_dispatch() {
    let _env = EnvGuard::set(&[(
        "HERDR_PLUGIN_EVENT_JSON",
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"w1:t1",
            "workspace_id":"w1",
            "label":"Renamed"
          }
        }"#,
    )]);
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let app = App::builder()
        .build()
        .unwrap()
        .on_event::<TabRenamed>({
            let calls = calls.clone();
            move |_ctx: Context, event: TabRenamed| {
                let calls = calls.clone();
                async move {
                    calls.lock().await.push(format!("event:{}", event.label));
                }
            }
        })
        .teardown({
            let calls = calls.clone();
            move |_ctx: Context| {
                let calls = calls.clone();
                async move {
                    calls.lock().await.push("teardown".to_string());
                    Ok(())
                }
            }
        });

    app.run().await.unwrap();

    assert_eq!(*calls.lock().await, ["event:Renamed", "teardown"]);
}

#[tokio::test]
async fn on_error_runs_when_setup_fails() {
    let _env = EnvGuard::set(&[]);
    let errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let app = App::builder()
        .build()
        .unwrap()
        .setup(|_ctx: Context| async {
            Err::<(), herdr_plugin::SetupError>(
                std::io::Error::new(std::io::ErrorKind::Other, "setup failed").into(),
            )
        })
        .on_error({
            let errors = errors.clone();
            move |_ctx: Context, error: String| {
                let errors = errors.clone();
                async move {
                    errors.lock().await.push(error);
                }
            }
        });

    let error = app.run().await.unwrap_err();

    assert!(matches!(error, RuntimeError::Setup { .. }));
    assert_eq!(*errors.lock().await, ["setup callback failed"]);
}

#[tokio::test]
async fn app_new_reads_herdr_runtime_environment_into_context() {
    let _env = EnvGuard::set(&[
        ("HERDR_SOCKET_PATH", "/tmp/herdr.sock"),
        ("HERDR_BIN_PATH", "/opt/herdr/bin/herdr"),
        ("HERDR_ENV", "1"),
        ("HERDR_PLUGIN_ID", "sample-plugin"),
        ("HERDR_PLUGIN_ROOT", "/plugins/sample"),
        ("HERDR_PLUGIN_CONFIG_DIR", "/config/sample"),
        ("HERDR_PLUGIN_STATE_DIR", "/state/sample"),
        (
            "HERDR_PLUGIN_CONTEXT_JSON",
            r#"{
              "workspace_id":"w1",
              "workspace_label":"Workspace",
              "workspace_cwd":"/repo",
              "tab_id":"w1:t1",
              "tab_label":"Tab",
              "focused_pane_id":"w1:p1",
              "focused_pane_cwd":"/repo",
              "focused_pane_agent":"codex",
              "focused_pane_status":"working",
              "selected_text":"hello",
              "invocation_source":"event",
              "correlation_id":"corr-1",
              "clicked_url":"https://example.com",
              "link_handler_id":"open-link"
            }"#,
        ),
        ("HERDR_WORKSPACE_ID", "w1"),
        ("HERDR_TAB_ID", "w1:t1"),
        ("HERDR_PANE_ID", "w1:p1"),
        ("HERDR_PLUGIN_ACTION_ID", "action-1"),
        ("HERDR_PLUGIN_EVENT", "tab.renamed"),
        (
            "HERDR_PLUGIN_EVENT_JSON",
            r#"{
              "event":"tab_renamed",
              "data":{
                "type":"tab_renamed",
                "tab_id":"w1:t1",
                "workspace_id":"w1",
                "label":"New tab"
              }
            }"#,
        ),
        ("HERDR_PLUGIN_ENTRYPOINT_ID", "pane-command"),
    ]);
    let captured = Arc::new(Mutex::new(None));
    let app = App::builder().build().unwrap().on_event::<TabRenamed>({
        let captured = captured.clone();
        move |ctx: Context, _event: TabRenamed| {
            let captured = captured.clone();
            async move {
                *captured.lock().await = Some(ctx);
            }
        }
    });

    app.run().await.unwrap();

    let ctx = captured.lock().await.clone().unwrap();
    assert!(ctx.env().is_herdr);
    assert_eq!(
        ctx.env().socket_path.as_deref(),
        Some(Path::new("/tmp/herdr.sock"))
    );
    assert_eq!(
        ctx.env().bin_path.as_deref(),
        Some(Path::new("/opt/herdr/bin/herdr"))
    );
    assert_eq!(ctx.env().plugin_id.as_deref(), Some("sample-plugin"));
    assert_eq!(
        ctx.env().plugin_root.as_deref(),
        Some(Path::new("/plugins/sample"))
    );
    assert_eq!(
        ctx.env().plugin_config_dir.as_deref(),
        Some(Path::new("/config/sample"))
    );
    assert_eq!(
        ctx.env().plugin_state_dir.as_deref(),
        Some(Path::new("/state/sample"))
    );
    assert_eq!(ctx.env().workspace_id.as_deref(), Some("w1"));
    assert_eq!(ctx.env().tab_id.as_deref(), Some("w1:t1"));
    assert_eq!(ctx.env().pane_id.as_deref(), Some("w1:p1"));
    assert_eq!(ctx.env().plugin_action_id.as_deref(), Some("action-1"));
    assert_eq!(ctx.env().plugin_event.as_deref(), Some("tab.renamed"));
    assert_eq!(
        ctx.env().plugin_entrypoint_id.as_deref(),
        Some("pane-command")
    );

    let plugin_context: PluginInvocationContext = ctx.env().plugin_context.clone().unwrap();
    assert_eq!(plugin_context.workspace_id.as_deref(), Some("w1"));
    assert_eq!(plugin_context.focused_pane_agent.as_deref(), Some("codex"));
    assert_eq!(plugin_context.selected_text.as_deref(), Some("hello"));
    assert_eq!(
        plugin_context.clicked_url.as_deref(),
        Some("https://example.com")
    );

    let event = ctx.env().plugin_event_json.clone().unwrap();
    assert_eq!(event.event, EventKind::TabRenamed);
}

#[tokio::test]
async fn run_returns_typed_error_for_invalid_event_json() {
    let _env = EnvGuard::set(&[("HERDR_PLUGIN_EVENT_JSON", "not-json")]);
    let app = App::builder().build().unwrap();
    let error = app.run().await.unwrap_err();

    assert!(matches!(error, RuntimeError::InvalidEventJson { .. }));
}

#[test]
fn env_event_source_reads_typed_runtime_event_from_env() {
    let _env = EnvGuard::set(&[(
        "HERDR_PLUGIN_EVENT_JSON",
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"w1:t1",
            "workspace_id":"w1",
            "label":"Renamed"
          }
        }"#,
    )]);

    let output = EnvEventSource::from_env().unwrap();

    assert!(matches!(
        output.event,
        Some(RuntimeEvent::TabRenamed(TabRenamed { label, .. })) if label == "Renamed"
    ));
    assert!(output.env.plugin_event_json.is_some());
}

#[test]
fn typed_herdr_events_implement_runtime_event_trait() {
    fn assert_event<E: Event>() {}

    assert_event::<TabRenamed>();
}
