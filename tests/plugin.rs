use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use herdr_plugin::{App, Context, HerdrClient, Plugin, TabRenamed};
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

#[derive(Clone)]
struct SearchPlugin {
    seen: Arc<TokioMutex<Vec<String>>>,
}

impl Plugin for SearchPlugin {
    fn build(&self, app: &mut App) {
        app.on::<TabRenamed>({
            let seen = self.seen.clone();
            move |_ctx: Context, event: TabRenamed| {
                let seen = seen.clone();
                async move {
                    seen.lock().await.push(event.label);
                }
            }
        });
    }
}

#[tokio::test]
async fn plugin_build_registers_handlers_on_the_app() {
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

    SearchPlugin { seen: seen.clone() }.build(&mut app);
    app.run().await.unwrap();

    assert_eq!(*seen.lock().await, ["renamed"]);
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
            assert_eq!(ctx.state().value, 7);
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
