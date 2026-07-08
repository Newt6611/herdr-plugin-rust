use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use herdr_plugin::{
    App, Context, HerdrClient, PluginInstallOptions, PluginListOptions, TabRenamed,
};
use serde::Deserialize;
use tokio::sync::Mutex as TokioMutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct EnvGuard {
    previous: Vec<(&'static str, Option<OsString>)>,
    _guard: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn event_json(json: &str) -> Self {
        Self::set(&[("HERDR_PLUGIN_EVENT_JSON", json)])
    }

    fn set(vars: &[(&'static str, &str)]) -> Self {
        let guard = ENV_LOCK.lock().unwrap();
        let keys = ["HERDR_PLUGIN_EVENT_JSON", "HERDR_PLUGIN_CONFIG_DIR"];
        let previous = keys
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect::<Vec<(&str, Option<OsString>)>>();

        for (key, _) in &previous {
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

fn temp_config_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("herdr-plugin-test-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(unix)]
fn fake_herdr(script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FAKE_ID: AtomicU64 = AtomicU64::new(0);

    let id = NEXT_FAKE_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "herdr-plugin-client-test-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let path = dir.join("herdr");
    fs::write(&path, script).unwrap();

    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();

    path
}

fn script(body: &str) -> String {
    format!("#!/bin/sh\nset -eu\n{body}\n")
}

#[cfg(unix)]
#[tokio::test]
async fn plugin_management_methods_use_expected_arguments() {
    let herdr = fake_herdr(&script(
        r#"case "$*" in
  "plugin install owner/repo/tool --ref main --yes")
    printf '%s\n' 'Installed example.tool from owner/repo/tool.'
    ;;
  "plugin list --json --plugin example.tool")
    printf '%s\n' '{"id":"cli:plugin","result":{"plugins":[{"actions":[],"build":[],"enabled":true,"events":[],"link_handlers":[],"manifest_path":"/plugins/example/herdr-plugin.toml","min_herdr_version":"0.7.0","name":"Example Tool","panes":[],"plugin_id":"example.tool","plugin_root":"/plugins/example","source":{"kind":"local"},"version":"0.1.0","warnings":[]}],"type":"plugin_list"}}'
    ;;
  "plugin enable example.tool")
    printf '%s\n' '{"id":"cli:plugin","result":{"plugin":{"actions":[],"build":[],"enabled":true,"events":[],"link_handlers":[],"manifest_path":"/plugins/example/herdr-plugin.toml","min_herdr_version":"0.7.0","name":"Example Tool","panes":[],"plugin_id":"example.tool","plugin_root":"/plugins/example","source":{"kind":"local"},"version":"0.1.0","warnings":[]},"type":"plugin_enabled"}}'
    ;;
  "plugin disable example.tool")
    printf '%s\n' '{"id":"cli:plugin","result":{"plugin":{"actions":[],"build":[],"enabled":false,"events":[],"link_handlers":[],"manifest_path":"/plugins/example/herdr-plugin.toml","min_herdr_version":"0.7.0","name":"Example Tool","panes":[],"plugin_id":"example.tool","plugin_root":"/plugins/example","source":{"kind":"local"},"version":"0.1.0","warnings":[]},"type":"plugin_disabled"}}'
    ;;
  "plugin uninstall example.tool")
    printf '%s\n' 'Uninstalled example.tool.'
    ;;
  *) exit 99 ;;
esac"#,
    ));
    let client = HerdrClient::with_binary(herdr);

    client
        .plugin()
        .install(PluginInstallOptions {
            source: "owner/repo/tool".to_owned(),
            requested_ref: Some("main".to_owned()),
            yes: true,
        })
        .await
        .unwrap();

    let list = client
        .plugin()
        .list(PluginListOptions {
            plugin_id: Some("example.tool".to_owned()),
        })
        .await
        .unwrap();
    assert_eq!(list.plugins[0].plugin_id, "example.tool");

    assert!(
        client
            .plugin()
            .enable("example.tool")
            .await
            .unwrap()
            .plugin
            .enabled
    );
    assert!(
        !client
            .plugin()
            .disable("example.tool")
            .await
            .unwrap()
            .plugin
            .enabled
    );
    client.plugin().uninstall("example.tool").await.unwrap();
}

#[tokio::test]
async fn app_registers_handlers_directly() {
    let _env = EnvGuard::event_json(
        r#"{
          "event":"tab_renamed",
          "data":{
            "type":"tab_renamed",
            "tab_id":"wT:t1",
            "workspace_id":"wT",
            "label":"renamed"
          }
        }"#,
    );
    let seen = Arc::new(TokioMutex::new(Vec::<String>::new()));
    let mut app = App::builder().build().unwrap();

    app.on::<TabRenamed>({
        let seen = seen.clone();
        move |_ctx: Context, event: TabRenamed| {
            let seen = seen.clone();
            async move {
                seen.lock().await.push(event.label);
            }
        }
    });
    app.run().await.unwrap();

    assert_eq!(*seen.lock().await, ["renamed"]);
}

#[tokio::test]
async fn app_direct_registration_supports_typed_config_context() {
    #[derive(Debug, Default, Deserialize)]
    struct Config {
        label_prefix: String,
    }

    let config_dir = temp_config_dir("plugin-config");
    fs::write(config_dir.join("config.toml"), r#"label_prefix = "cfg""#).unwrap();
    let _env = EnvGuard::set(&[
        ("HERDR_PLUGIN_CONFIG_DIR", config_dir.to_str().unwrap()),
        (
            "HERDR_PLUGIN_EVENT_JSON",
            r#"{
              "event":"tab_renamed",
              "data":{
                "type":"tab_renamed",
                "tab_id":"wT:t1",
                "workspace_id":"wT",
                "label":"renamed"
              }
            }"#,
        ),
    ]);

    let seen = Arc::new(TokioMutex::new(Vec::<String>::new()));
    let mut app = App::builder().with_config::<Config>().build().unwrap();

    app.on::<TabRenamed>({
        let seen = seen.clone();
        move |ctx: Context<(), Config>, event: TabRenamed| {
            let seen = seen.clone();
            async move {
                seen.lock()
                    .await
                    .push(format!("{}:{}", ctx.config().label_prefix, event.label));
            }
        }
    });
    app.run().await.unwrap();

    assert_eq!(*seen.lock().await, ["cfg:renamed"]);
}

#[test]
fn plugin_surface_reexports_herdr_client_for_app_builders() {
    let client = Arc::new(HerdrClient::with_binary("/definitely/missing/herdr"));
    let _app = App::builder().with_client(client).build().unwrap();
}

#[test]
fn plugin_surface_exposes_setup_hook() {
    let _app = App::builder()
        .build()
        .unwrap()
        .setup(|_ctx: Context| async { Ok(()) });
}

#[test]
fn plugin_surface_exposes_typed_app_state() {
    #[derive(Debug)]
    struct State {
        value: usize,
    }

    let _app = App::builder()
        .with_state(State { value: 7 })
        .build()
        .unwrap()
        .setup(|ctx: Context<State>| async move {
            let state = ctx.state();
            assert_eq!(state.value, 7);
            Ok(())
        });
}

#[test]
fn plugin_surface_exposes_typed_config() {
    #[derive(Debug, Default, Deserialize)]
    struct Config {
        label_prefix: String,
    }

    let config_dir = temp_config_dir("typed-config");
    let _env = EnvGuard::set(&[("HERDR_PLUGIN_CONFIG_DIR", config_dir.to_str().unwrap())]);

    let _app = App::builder()
        .with_config::<Config>()
        .build()
        .unwrap()
        .setup(|ctx: Context<(), Config>| async move {
            let _ = &ctx.config().label_prefix;
            Ok(())
        });
}

#[test]
fn plugin_surface_exposes_context_helpers_and_lifecycle_hooks() {
    let _app = App::builder()
        .build()
        .unwrap()
        .setup(|ctx: Context| async move {
            ctx.log().info("setup");
            let _ = ctx.config_path("config.toml");
            let _ = ctx.state_path("state.json");
            let _ = ctx.event_kind();
            Ok(())
        })
        .teardown(|_ctx: Context| async { Ok(()) })
        .on_error(|ctx: Context, error: String| async move {
            ctx.log().error(error);
        });
}
