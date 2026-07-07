use herdr_plugin::{App, Context, PaneFocused, TabCreated, WorkspaceCreated};

struct PluginState {
    label_prefix: String,
}

async fn setup(ctx: Context<PluginState>) -> Result<(), herdr_plugin::SetupError> {
    println!("plugin id: {:?}", ctx.env().plugin_id);
    println!("label prefix: {}", ctx.state().label_prefix);

    let tabs = ctx.client().tab().list(Default::default()).await?;
    println!("current tab count: {}", tabs.tabs.len());

    Ok(())
}

async fn tab_created(ctx: Context<PluginState>, event: TabCreated) {
    println!(
        "{} tab created: {}",
        ctx.state().label_prefix,
        event.tab.tab_id
    );
}

async fn pane_focused(_ctx: Context<PluginState>, event: PaneFocused) {
    println!("pane focused: {}", event.pane_id);
}

async fn workspace_created(_ctx: Context<PluginState>, event: WorkspaceCreated) {
    println!("workspace created: {}", event.workspace.workspace_id);
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
        .run()
        .await
}
