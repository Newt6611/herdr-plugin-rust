# herdr-plugin-rs

Rust SDK and runtime building blocks for Herdr plugins.

`herdr-plugin-rs` does not replace Herdr's plugin system. It provides a typed,
ergonomic Rust layer on top of Herdr's existing manifest, CLI, and environment
contracts.

## Repository Description

Rust SDK and runtime for building Herdr plugins.

## Status

This project is early and intentionally incremental. The current focus is:

- strongly typed runtime event handlers
- workspace, tab, and pane event types
- Herdr runtime environment capture
- a CLI-backed `HerdrClient`
- resource clients for sessions, workspaces, worktrees, tabs, panes, and agents
- socket transport support planned

The published crate is `herdr-plugin`.

## Workspace

```text
crates/
  plugin/      published SDK crate
  dispatcher/  unpublished reference/internal crate
  runtime/     unpublished reference/internal crate
  client/      unpublished reference/internal crate
examples/
  minimal/     minimal plugin runtime example
```

Install the SDK with:

```toml
[dependencies]
herdr-plugin = "0.1.2"
```

## Runtime Example

```rust
use herdr_plugin::{App, Context, PaneFocused, TabCreated, WorkspaceCreated};

async fn setup(ctx: Context) -> Result<(), herdr_plugin::SetupError> {
    println!("plugin id: {:?}", ctx.env().plugin_id);

    let tabs = ctx.client().tab().list(Default::default()).await?;
    println!("current tab count: {}", tabs.tabs.len());

    Ok(())
}

async fn tab_created(ctx: Context, event: TabCreated) {
    println!("tab created: {}", event.tab.tab_id);

    let tabs = ctx.client().tab().list(Default::default()).await;
    println!("tabs: {tabs:?}");
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
```

Plugin-style registration is also supported:

```rust
use herdr_plugin::{App, Context, Plugin, TabRenamed};

struct SearchPlugin;

impl Plugin for SearchPlugin {
    fn build(&self, app: &mut App) {
        app.on::<TabRenamed>(tab_renamed);
    }
}

async fn tab_renamed(_ctx: Context, event: TabRenamed) {
    println!("renamed to {}", event.label);
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    let mut app = App::new();
    SearchPlugin.build(&mut app);
    app.run().await
}
```

## Setup Hooks

Use `App::setup(...)` when your plugin needs to load state, inspect the Herdr
runtime environment, or fetch data before handling the current event.

Setup callbacks receive the same `Context` as event handlers. They run after
`App::run()` reads Herdr environment variables and before the current event
starts dispatching.

```rust
use herdr_plugin::{App, Context};

async fn setup(ctx: Context) -> Result<(), herdr_plugin::SetupError> {
    let workspaces = ctx.client().workspace().list().await?;
    println!("workspaces: {}", workspaces.workspaces.len());
    Ok(())
}

let app = App::new().setup(setup);
```

## Context

Every event handler receives a `Context`:

```rust
pub struct Context {
    // fields are private
}
```

`ctx.client()` returns the shared typed client for calling Herdr commands.

`ctx.env()` returns Herdr-provided runtime values such as:

- `HERDR_SOCKET_PATH`
- `HERDR_BIN_PATH`
- `HERDR_PLUGIN_ID`
- `HERDR_PLUGIN_ROOT`
- `HERDR_PLUGIN_CONFIG_DIR`
- `HERDR_PLUGIN_STATE_DIR`
- `HERDR_WORKSPACE_ID`
- `HERDR_TAB_ID`
- `HERDR_PANE_ID`
- `HERDR_PLUGIN_CONTEXT_JSON`
- `HERDR_PLUGIN_EVENT_JSON`

`App::run()` reads from an internal environment event source, converts
`HERDR_PLUGIN_EVENT_JSON` into the matching typed event payload, and dispatches
it to registered handlers. This keeps event parsing separate from the runtime
dispatcher so socket, replay, and testing sources can be added later.

## Client Example

```rust
use herdr_plugin::HerdrClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HerdrClient::new();

    let sessions = client.session().list().await?;
    println!("{sessions:?}");

    let workspaces = client.workspace().list().await?;
    println!("{workspaces:?}");

    Ok(())
}
```

`HerdrClient::new()` uses `HERDR_BIN_PATH` when set and falls back to `herdr`.
You can override the binary explicitly:

```rust
use herdr_plugin::{App, HerdrClient};

let client = HerdrClient::with_binary("/path/to/herdr");
let app = App::with_client(std::sync::Arc::new(client));
```

Or configure the runtime builder directly:

```rust
let app = App::new().with_herdr_bin_path("/path/to/herdr");
```

## Design Notes

- `herdr-plugin` is the single published SDK crate.
- The generic dispatcher, runtime, and client implementations live inside
  `herdr-plugin` so users only depend on one crate.
- The client currently shells out to the local `herdr` binary with
  `tokio::process::Command`.
- Socket transport support is planned, but not implemented yet.

The architecture is deliberately small so middleware, filters, handler priority,
dynamic registration, and socket transport support can be added later without
rewriting the public API.

## Development

Run the full workspace test suite:

```sh
cargo test --workspace
```

Format all crates:

```sh
cargo fmt --all
```
