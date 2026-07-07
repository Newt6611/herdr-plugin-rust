use std::{
    ffi::OsString,
    sync::{Arc, Mutex, MutexGuard},
};

use herdr_plugin::{App, Context, HerdrClient, Plugin, TabRenamed};
use tokio::sync::Mutex as TokioMutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct EnvGuard {
    previous: Option<OsString>,
    _guard: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn event_json(json: &str) -> Self {
        let guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os("HERDR_PLUGIN_EVENT_JSON");
        std::env::set_var("HERDR_PLUGIN_EVENT_JSON", json);
        Self {
            previous,
            _guard: guard,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.previous.as_ref() {
            Some(value) => std::env::set_var("HERDR_PLUGIN_EVENT_JSON", value),
            None => std::env::remove_var("HERDR_PLUGIN_EVENT_JSON"),
        }
    }
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
    let mut app = App::new();

    SearchPlugin { seen: seen.clone() }.build(&mut app);
    app.run().await.unwrap();

    assert_eq!(*seen.lock().await, ["renamed"]);
}

#[test]
fn plugin_surface_reexports_herdr_client_for_app_builders() {
    let client = Arc::new(HerdrClient::with_binary("/definitely/missing/herdr"));
    let _app = App::with_client(client);
}

#[test]
fn plugin_surface_exposes_setup_hook() {
    let _app = App::new().setup(|_ctx: Context| async { Ok(()) });
}
