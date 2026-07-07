use herdr_plugin::{App, Context, PaneFocused, TabCreated, WorkspaceCreated};
use serde::Deserialize;

struct PluginState {
    run_label: String,
}

#[derive(Debug, Default, Deserialize)]
struct PluginConfig {
    label_prefix: String,
}

async fn setup(ctx: Context<PluginState, PluginConfig>) -> Result<(), herdr_plugin::SetupError> {
    ctx.log()
        .info(format!("plugin id: {:?}", ctx.env().plugin_id));
    ctx.log()
        .info(format!("run label: {}", ctx.state().run_label));
    ctx.log()
        .info(format!("label prefix: {}", ctx.config().label_prefix));
    ctx.log()
        .info(format!("config path: {:?}", ctx.config_path("config.toml")));

    let tabs = ctx.client().tab().list(Default::default()).await?;
    ctx.log()
        .info(format!("current tab count: {}", tabs.tabs.len()));

    Ok(())
}

async fn tab_created(ctx: Context<PluginState, PluginConfig>, event: TabCreated) {
    ctx.log().info(format!(
        "{} tab created: {}",
        ctx.config().label_prefix,
        event.tab.tab_id
    ));
}

async fn pane_focused(ctx: Context<PluginState, PluginConfig>, event: PaneFocused) {
    ctx.log().info(format!("pane focused: {}", event.pane_id));
}

async fn workspace_created(ctx: Context<PluginState, PluginConfig>, event: WorkspaceCreated) {
    ctx.log().info(format!(
        "workspace created: {}",
        event.workspace.workspace_id
    ));
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    App::builder()
        .with_state(PluginState {
            run_label: "minimal".to_string(),
        })
        .with_config::<PluginConfig>()
        .build()?
        .setup(setup)
        .on_event::<TabCreated>(tab_created)
        .on_event::<PaneFocused>(pane_focused)
        .on_event::<WorkspaceCreated>(workspace_created)
        .teardown(|ctx: Context<PluginState, PluginConfig>| async move {
            ctx.log().info("plugin invocation finished");
            Ok(())
        })
        .on_error(
            |ctx: Context<PluginState, PluginConfig>, error| async move {
                ctx.log().error(error);
            },
        )
        .run()
        .await
}
