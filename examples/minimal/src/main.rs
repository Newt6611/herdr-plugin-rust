use herdr_plugin::{App, Context, PaneFocused, TabCreated, WorkspaceCreated};

async fn setup(ctx: Context) -> Result<(), herdr_plugin::SetupError> {
    println!("plugin id: {:?}", ctx.env().plugin_id);

    let tabs = ctx.client().tab().list(Default::default()).await?;
    println!("current tab count: {}", tabs.tabs.len());

    Ok(())
}

async fn tab_created(_ctx: Context, event: TabCreated) {
    println!("tab created: {}", event.tab.tab_id);
}

async fn pane_focused(_ctx: Context, event: PaneFocused) {
    println!("pane focused: {}", event.pane_id);
}

async fn workspace_created(_ctx: Context, event: WorkspaceCreated) {
    println!("workspace created: {}", event.workspace.workspace_id);
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    App::new()
        .setup(setup)
        .on_event::<TabCreated>(tab_created)
        .on_event::<PaneFocused>(pane_focused)
        .on_event::<WorkspaceCreated>(workspace_created)
        .run()
        .await
}
