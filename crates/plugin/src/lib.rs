//! Rust SDK surface for Herdr plugins.

pub use herdr_client::HerdrClient;
pub use herdr_dispatcher::{EventDispatcher, Handler};
pub use herdr_runtime::env;
pub use herdr_runtime::env::*;
pub use herdr_runtime::event_source;
pub use herdr_runtime::event_source::*;
pub use herdr_runtime::events;
pub use herdr_runtime::events::*;
pub use herdr_runtime::{App, Context, RuntimeError};

/// A Herdr plugin module that registers event handlers on an application.
pub trait Plugin {
    fn build(&self, app: &mut App);
}
