# herdr-plugin-rs

Rust SDK and runtime building blocks for Herdr plugins.

`herdr-plugin-rs` does not replace Herdr's plugin system. It provides a typed,
ergonomic Rust layer on top of Herdr's existing manifest, CLI, environment, and
event hook contracts.

The published crate is `herdr-plugin`.

## Status

This project is early and intentionally incremental.

Currently included:

- typed runtime event handlers
- workspace, tab, and pane event types
- typed config loading from `HERDR_PLUGIN_CONFIG_DIR`
- runtime state through `Context`
- a CLI-backed `HerdrClient`
- resource clients for sessions, workspaces, worktrees, tabs, panes, and agents

Current Herdr integration is CLI-only: `HerdrClient` shells out to the local
`herdr` binary, using `HERDR_BIN_PATH` when Herdr provides it. Direct socket
integration through `HERDR_SOCKET_PATH` is planned for a future release.

## Install

```sh
cargo add herdr-plugin serde --features serde/derive
cargo add tokio --features macros,rt-multi-thread # if using #[tokio::main]
```

```toml
[dependencies]
herdr-plugin = "0.1.4"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] } # if using #[tokio::main]
```

The SDK is async and currently uses Tokio internally for CLI process handling.
Plugin binaries still need to choose an executor; the examples use Tokio.

## Example

```rust
use herdr_plugin::{App, Context, TabCreated};
use serde::Deserialize;

#[derive(Debug)]
struct State {
    prefix: String,
    seen_tabs: usize,
}

#[derive(Debug, Default, Deserialize)]
struct Config {
    label_prefix: String,
}

async fn setup(ctx: Context<State, Config>) -> herdr_plugin::SetupResult {
    let tabs = ctx.client().tab().list(Default::default()).await?;
    ctx.state_mut().seen_tabs = tabs.tabs.len();
    ctx.log().info("plugin initialized");
    Ok(())
}

async fn tab_created(ctx: Context<State, Config>, event: TabCreated) {
    let prefix = {
        let state = ctx.state();
        state.prefix.clone()
    };

    ctx.log().info(format!(
        "{} {} tab created: {}",
        ctx.config().label_prefix,
        prefix,
        event.tab.tab_id
    ));
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    App::builder()
        .with_state(State {
            prefix: "tab".to_string(),
            seen_tabs: 0,
        })
        .with_config::<Config>()
        .build()?
        .setup(setup)
        .on_event::<TabCreated>(tab_created)
        .run()
        .await
}
```

## Runtime

`App::builder()` reads Herdr's runtime environment during `build()`, loads
optional typed config, creates the Herdr client, and prepares the event payload
from `HERDR_PLUGIN_EVENT_JSON`.

```rust
let app = App::builder()
    .with_config::<Config>()
    .with_herdr_bin_path("/path/to/herdr")
    .build()?;
```

Relative config paths are resolved under `HERDR_PLUGIN_CONFIG_DIR`:

```rust
let app = App::builder()
    .with_config_file::<Config>("settings.toml")
    .build()?;
```

Absolute config paths are used directly:

```rust
let app = App::builder()
    .with_config_path::<Config>("/tmp/plugin.toml")
    .build()?;
```

## Context

Every setup callback, teardown callback, error callback, and event handler
receives a `Context<State, Config>`.

Useful accessors:

```rust
ctx.client();
ctx.env();
ctx.config();
ctx.log();
ctx.state();
ctx.state_mut();
ctx.config_path("config.toml");
ctx.state_path("cache.json");
ctx.is_event();
ctx.is_action();
ctx.event_kind();
```

State is stored behind a mutex. Do not hold a `state()` or `state_mut()` guard
across `.await`; copy or clone the values you need first.

## Client

`HerdrClient::new()` uses `HERDR_BIN_PATH` when set and falls back to `herdr`.
The client currently calls Herdr through `tokio::process::Command`.

```rust
use herdr_plugin::HerdrClient;

let client = HerdrClient::new();
let sessions = client.session().list().await?;
let tabs = client.tab().list(Default::default()).await?;
```

## Layout

```text
src/           published SDK crate
tests/         integration tests
examples/
  minimal/     minimal runtime example
```

## Development

```sh
cargo test --workspace
cargo fmt --all
```
