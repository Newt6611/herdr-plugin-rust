use herdr_plugin::{
    App, Context, EventKind, PaneFocused, SocketRuntime, TabRenamed, WorkspaceCreateOptions,
};
use serde::Deserialize;

#[derive(Debug)]
struct PluginState {
    label: String,
}

#[derive(Debug, Default, Deserialize)]
struct PluginConfig {
    workspace_label: Option<String>,
}

async fn setup(ctx: Context<PluginState, PluginConfig>) -> herdr_plugin::SetupResult {
    ctx.log()
        .info(format!("socket path: {:?}", ctx.env().socket_path));
    ctx.log()
        .info(format!("state label: {}", ctx.state().label));
    Ok(())
}

async fn tab_renamed(ctx: Context<PluginState, PluginConfig>, event: TabRenamed) {
    ctx.log()
        .info(format!("tab renamed: {} -> {}", event.tab_id, event.label));
}

async fn pane_focused(ctx: Context<PluginState, PluginConfig>, event: PaneFocused) {
    ctx.log().info(format!("pane focused: {}", event.pane_id));
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    let runtime = SocketRuntime::new().subscribe([EventKind::TabRenamed, EventKind::PaneFocused]);
    let handle = runtime.handle();

    tokio::spawn(async move {
        if std::env::var_os("HERDR_SOCKET_EXAMPLE_CREATE_WORKSPACE").is_none() {
            return;
        }

        let result = handle
            .workspace()
            .create(WorkspaceCreateOptions {
                cwd: None,
                label: Some("socket-runtime-example".to_owned()),
                env: Vec::new(),
                focus: Some(false),
            })
            .await;

        match result {
            Ok(created) => {
                eprintln!("created workspace {}", created.workspace.workspace_id);
            }
            Err(error) => {
                eprintln!("workspace create failed: {error}");
            }
        }
    });

    App::builder()
        .runtime(runtime)
        .with_state(PluginState {
            label: "socket-runtime".to_owned(),
        })
        .with_config::<PluginConfig>()
        .build()?
        .setup(setup)
        .on_event::<TabRenamed>(tab_renamed)
        .on_event::<PaneFocused>(pane_focused)
        .teardown(|ctx: Context<PluginState, PluginConfig>| async move {
            if let Some(label) = ctx.config().workspace_label.as_deref() {
                ctx.log()
                    .info(format!("configured workspace label: {label}"));
            }
            ctx.log().info("socket runtime stopped");
            Ok(())
        })
        .run()
        .await
}
