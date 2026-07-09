# herdr-plugin-rs

A Rust application framework for building Herdr plugins.

The published crate is `herdr-plugin`.

## Install

```sh
cargo add herdr-plugin serde --features serde/derive
cargo add tokio --features macros,rt-multi-thread
```

```toml
[dependencies]
herdr-plugin = "0.1.7"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Example

```rust
use herdr_plugin::{App, Context, OneShotRuntime, TabCreated};
use serde::Deserialize;

#[derive(Debug, Default)]
struct State {
    seen_tabs: usize,
}

#[derive(Debug, Default, Deserialize)]
struct Config {
    label_prefix: String,
}

async fn setup(ctx: Context<State, Config>) -> herdr_plugin::SetupResult {
    let tabs = ctx.client().tab().list(Default::default()).await?;
    ctx.state_mut().seen_tabs = tabs.tabs.len();
    Ok(())
}

async fn tab_created(ctx: Context<State, Config>, event: TabCreated) {
    ctx.log().info(format!(
        "{} tab created: {}",
        ctx.config().label_prefix,
        event.tab.tab_id
    ));
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    App::builder()
        .runtime(OneShotRuntime::new())
        .with_state(State::default())
        .with_config::<Config>()
        .build()?
        .setup(setup)
        .on_event::<TabCreated>(tab_created)
        .run()
        .await
}
```

## Runtime

`OneShotRuntime` is the default runtime for normal Herdr plugin hooks. It reads
Herdr's environment, loads typed config, runs setup, dispatches one event, then
runs teardown.

`SocketRuntime` is for long-running plugins. `App::run().await` blocks while
the socket loop is alive. Socket runtime is still in testing.

```rust
use herdr_plugin::{App, Context, EventKind, SocketRuntime, TabRenamed};

async fn tab_renamed(ctx: Context, event: TabRenamed) {
    if let Some(socket) = ctx.socket() {
        let _ = socket.tab().focus(&event.tab_id).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), herdr_plugin::RuntimeError> {
    App::builder()
        .runtime(SocketRuntime::new().subscribe([EventKind::TabRenamed]))
        .build()?
        .on_event::<TabRenamed>(tab_renamed)
        .run()
        .await
}
```

## Context

Callbacks receive `Context<State, Config>` with access to:

```rust
ctx.client();      // CLI-backed Herdr client
ctx.socket();      // socket handle, only under SocketRuntime
ctx.env();
ctx.config();
ctx.state();
ctx.state_mut();
ctx.log();
```

Do not hold a `state()` or `state_mut()` guard across `.await`; clone the values
you need first.

## Development

```sh
cargo fmt --all
cargo test --workspace
```
