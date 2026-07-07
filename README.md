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
src/           published SDK crate
tests/         SDK integration tests
examples/
  minimal/     minimal plugin runtime example
```

Install the SDK with:

```toml
[dependencies]
herdr-plugin = "0.1.4"
serde = { version = "1", features = ["derive"] }
```

## Runtime Example

```rust
use herdr_plugin::{App, Context, PaneFocused, TabCreated, WorkspaceCreated};
use serde::Deserialize;

struct PluginState {
    run_label: String,
}

#[derive(Debug, Default, Deserialize)]
struct PluginConfig {
    label_prefix: String,
}

async fn setup(ctx: Context<PluginState, PluginConfig>) -> herdr_plugin::SetupResult {
    ctx.log().info(format!("plugin id: {:?}", ctx.env().plugin_id));
    ctx.log().info(format!("run label: {}", ctx.state().run_label));
    ctx.log()
        .info(format!("label prefix: {}", ctx.config().label_prefix));
    ctx.log()
        .info(format!("config path: {:?}", ctx.config_path("config.toml")));

    let tabs = ctx.client().tab().list(Default::default()).await?;
    ctx.log().info(format!("current tab count: {}", tabs.tabs.len()));

    Ok(())
}

async fn tab_created(ctx: Context<PluginState, PluginConfig>, event: TabCreated) {
    ctx.log().info(format!(
        "{} tab created: {}",
        ctx.config().label_prefix,
        event.tab.tab_id
    ));

    let tabs = ctx.client().tab().list(Default::default()).await;
    ctx.log().info(format!("tabs: {tabs:?}"));
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
        .on_error(|ctx: Context<PluginState, PluginConfig>, error| async move {
            ctx.log().error(error);
        })
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
    let mut app = App::builder().build()?;
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

async fn setup(ctx: Context) -> herdr_plugin::SetupResult {
    let workspaces = ctx.client().workspace().list().await?;
    println!("workspaces: {}", workspaces.workspaces.len());
    Ok(())
}

let app = App::builder().build()?.setup(setup);
```

## App State

Use `App::with_state(...)` to attach plugin-owned state to the runtime. Setup
callbacks and event handlers can read it through `ctx.state()`.
Call `with_state` before registering setup callbacks or event handlers.

```rust
use herdr_plugin::{App, Context, TabCreated};

struct State {
    title_prefix: String,
}

async fn tab_created(ctx: Context<State>, event: TabCreated) {
    println!("{} {}", ctx.state().title_prefix, event.tab.tab_id);
}

let app = App::builder()
    .with_state(State {
        title_prefix: "tab".to_string(),
    })
    .build()?
    .on_event::<TabCreated>(tab_created);
```

State is immutable from the framework's point of view. If a plugin needs shared
mutable state, put a `Mutex`, `RwLock`, or atomic value inside the state struct.

## Config

Use `App::builder().with_config::<T>()` to load typed TOML config before setup
callbacks and event handlers run. By default, the runtime reads:

```text
$HERDR_PLUGIN_CONFIG_DIR/config.toml
```

Missing config files use `T::default()`. Invalid TOML returns
`RuntimeError::Config`.

```rust
use herdr_plugin::{App, Context, TabCreated};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
struct Config {
    title_prefix: String,
}

async fn tab_created(ctx: Context<(), Config>, event: TabCreated) {
    ctx.log().info(format!(
        "{} {}",
        ctx.config().title_prefix,
        event.tab.tab_id
    ));
}

let app = App::builder()
    .with_config::<Config>()
    .build()?
    .on_event::<TabCreated>(tab_created);
```

Custom relative paths are resolved under `HERDR_PLUGIN_CONFIG_DIR`:

```rust
let app = App::builder()
    .with_config_file::<Config>("settings.toml")
    .build()?;
```

Absolute paths are used directly:

```rust
let app = App::builder()
    .with_config_path::<Config>("/tmp/plugin.toml")
    .build()?;
```

## Lifecycle Hooks

`setup` runs after Herdr environment parsing and before the current event starts
dispatching. `teardown` runs after event dispatch completes. `on_error` runs
before `run()` returns a runtime error.

```rust
use herdr_plugin::{App, Context};

let app = App::builder()
    .build()?
    .setup(|ctx: Context| async move {
        ctx.log().info("starting");
        Ok(())
    })
    .teardown(|ctx: Context| async move {
        ctx.log().info("finished");
        Ok(())
    })
    .on_error(|ctx: Context, error| async move {
        ctx.log().error(error);
    });
```

## Logger

Use `ctx.log()` for simple runtime logging. The current implementation writes to
stderr; the API is intentionally small so it can later route through Herdr's
socket/logging system.

```rust
ctx.log().info("refresh started");
ctx.log().warn("refresh skipped");
ctx.log().error("refresh failed");
```

## Runtime Helpers

`Context` exposes small helpers for common Herdr runtime questions and paths:

```rust
ctx.is_event();
ctx.is_action();
ctx.event_kind();
ctx.config_dir();
ctx.state_dir();
ctx.config_path("config.toml");
ctx.state_path("cache.json");
```

## Context

Every event handler receives a `Context`:

```rust
pub struct Context<State = ()> {
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
let app = App::builder()
    .with_client(std::sync::Arc::new(client))
    .build()?;
```

Or configure the runtime builder directly:

```rust
let app = App::builder()
    .with_herdr_bin_path("/path/to/herdr")
    .build()?;
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
