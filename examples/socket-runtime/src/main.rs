use herdr_plugin::{
    App, Context, EventKind, PaneFocused, RuntimeHandleError, SocketRuntime, TabRenamed,
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

    if let Some(socket) = ctx.socket() {
        let _ = socket.tab().focus(&event.tab_id).await;
    }
}

async fn pane_focused(ctx: Context<PluginState, PluginConfig>, event: PaneFocused) {
    ctx.log().info(format!("pane focused: {}", event.pane_id));
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    let runtime = SocketRuntime::new().subscribe([EventKind::TabRenamed, EventKind::PaneFocused]);
    let handle = runtime.handle();

    let app = App::builder()
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
        .run();

    let app_task = tokio::spawn(app);

    while let Err(RuntimeHandleError::SocketUnavailable) = handle.server().ping().await {
        tokio::task::yield_now().await;
    }

    let _created = handle
        .workspace()
        .create(herdr_plugin::WorkspaceCreateOptions {
            cwd: None,
            label: Some("socket-runtime-handle-example".to_owned()),
            env: Vec::new(),
            focus: Some(false),
        })
        .await;

    handle.stop().await.expect("failed to stop socket runtime");

    app_task.await.expect("socket runtime task panicked")
}
