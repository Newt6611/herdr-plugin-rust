use herdr_plugin::{App, Context, PaneFocused, TabCreated, WorkspaceCreated};

struct PluginState {
    label_prefix: String,
}

async fn setup(ctx: Context<PluginState>) -> Result<(), herdr_plugin::SetupError> {
    ctx.log()
        .info(format!("plugin id: {:?}", ctx.env().plugin_id));
    ctx.log()
        .info(format!("label prefix: {}", ctx.state().label_prefix));
    ctx.log()
        .info(format!("config path: {:?}", ctx.config_path("config.toml")));

    let tabs = ctx.client().tab().list(Default::default()).await?;
    ctx.log()
        .info(format!("current tab count: {}", tabs.tabs.len()));

    Ok(())
}

async fn tab_created(ctx: Context<PluginState>, event: TabCreated) {
    ctx.log().info(format!(
        "{} tab created: {}",
        ctx.state().label_prefix,
        event.tab.tab_id
    ));
}

async fn pane_focused(ctx: Context<PluginState>, event: PaneFocused) {
    ctx.log().info(format!("pane focused: {}", event.pane_id));
}

async fn workspace_created(ctx: Context<PluginState>, event: WorkspaceCreated) {
    ctx.log().info(format!(
        "workspace created: {}",
        event.workspace.workspace_id
    ));
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    App::new()
        .with_state(PluginState {
            label_prefix: "search".to_string(),
        })
        .setup(setup)
        .on_event::<TabCreated>(tab_created)
        .on_event::<PaneFocused>(pane_focused)
        .on_event::<WorkspaceCreated>(workspace_created)
        .teardown(|ctx: Context<PluginState>| async move {
            ctx.log().info("plugin invocation finished");
            Ok(())
        })
        .on_error(|ctx: Context<PluginState>, error| async move {
            ctx.log().error(error);
        })
        .run()
        .await
}
