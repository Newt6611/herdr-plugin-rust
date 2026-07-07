use herdr_plugin::{App, Context, PaneFocused, Plugin, TabCreated, WorkspaceCreated};

struct SearchPlugin;

impl Plugin for SearchPlugin {
    fn build(&self, app: &mut App) {
        app.on::<TabCreated>(tab_created);
        app.on::<PaneFocused>(pane_focused);
        app.on::<WorkspaceCreated>(workspace_created);
    }
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
    let mut app = App::new();

    SearchPlugin.build(&mut app);

    app.run().await
}
