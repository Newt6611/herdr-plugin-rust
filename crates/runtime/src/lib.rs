//! Runtime app facade for Herdr plugins.

pub mod context;
pub mod env;
pub mod event_source;
pub mod events;

use std::{error::Error, future::Future, path::PathBuf, pin::Pin, sync::Arc};

use herdr_client::HerdrClient;
use herdr_dispatcher::{EventDispatcher, Handler};

pub use context::Context;
pub use env::{HerdrEnv, PluginInvocationContext};
use event_source::EnvEventSource;
use events::Event;

pub type SetupError = Box<dyn Error + Send + Sync + 'static>;
type SetupFuture = Pin<Box<dyn Future<Output = Result<(), SetupError>> + Send + 'static>>;
type SetupHandler<State> = Box<dyn Fn(Context<State>) -> SetupFuture + Send + Sync + 'static>;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("invalid HERDR_PLUGIN_EVENT_JSON")]
    InvalidEventJson {
        json: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("setup callback failed")]
    Setup {
        #[source]
        source: SetupError,
    },
}

/// Runtime application facade used to register and dispatch typed events.
pub struct App<State = ()> {
    context: Context<State>,
    dispatcher: EventDispatcher<Context<State>>,
    setup_handlers: Vec<SetupHandler<State>>,
    herdr_bin_path_override: Option<PathBuf>,
}

impl App<()> {
    /// Creates an app.
    pub fn new() -> Self {
        Self::with_client(HerdrClient::new())
    }

    /// Creates an app using an existing Herdr client handle.
    pub fn with_client(client: impl Into<Arc<HerdrClient>>) -> Self {
        Self {
            context: Context::new(client),
            dispatcher: EventDispatcher::default(),
            setup_handlers: Vec::new(),
            herdr_bin_path_override: None,
        }
    }
}

impl<State> App<State>
where
    State: Send + Sync + 'static,
{
    /// Attaches typed state that setup callbacks and event handlers can access through `Context`.
    ///
    /// Call this early in the builder chain, before registering setup callbacks
    /// or event handlers that need the state type.
    pub fn with_state<NextState>(self, state: NextState) -> App<NextState>
    where
        NextState: Send + Sync + 'static,
    {
        assert!(
            self.dispatcher.is_empty() && self.setup_handlers.is_empty(),
            "with_state must be called before registering setup callbacks or event handlers"
        );

        let state = Arc::new(state);

        App {
            context: Context::with_env_and_state(
                self.context.client_handle(),
                self.context.env().clone(),
                state,
            ),
            dispatcher: EventDispatcher::default(),
            setup_handlers: Vec::new(),
            herdr_bin_path_override: self.herdr_bin_path_override,
        }
    }

    /// Sets the Herdr binary path used by the client passed to event handlers.
    pub fn with_herdr_bin_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut env = self.context.env().clone();
        let client = Arc::new(HerdrClient::with_binary(path.clone()));
        self.herdr_bin_path_override = Some(path.clone());
        env.bin_path = Some(path);
        self.context = Context::with_env_and_state(client, env, self.context.state_handle());
        self
    }

    /// Registers an async handler for a concrete event type.
    pub fn on<E>(&mut self, handler: impl Handler<Context<State>, E>) -> &mut Self
    where
        E: Event,
    {
        self.dispatcher.on(handler);
        self
    }

    /// Registers an async event handler and returns the app for builder chaining.
    pub fn on_event<E>(mut self, handler: impl Handler<Context<State>, E>) -> Self
    where
        E: Event,
    {
        self.on(handler);
        self
    }

    /// Registers a setup callback that runs before the current event starts dispatching.
    ///
    /// Setup callbacks run after Herdr environment parsing, so they receive the
    /// same populated [`Context`] as event handlers.
    pub fn setup<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(Context<State>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), SetupError>> + Send + 'static,
    {
        self.setup_handlers
            .push(Box::new(move |context| Box::pin(handler(context))));
        self
    }

    /// Runs the app for the current Herdr plugin invocation.
    pub async fn run(mut self) -> Result<(), RuntimeError> {
        let mut output = EnvEventSource::from_env()?;
        if let Some(path) = self.herdr_bin_path_override.as_ref() {
            output.env.bin_path = Some(path.clone());
        }

        self.context = Context::with_env_and_state(
            self.context.client_handle(),
            output.env,
            self.context.state_handle(),
        );
        for handler in &self.setup_handlers {
            handler(self.context.clone())
                .await
                .map_err(|source| RuntimeError::Setup { source })?;
        }
        if let Some(event) = output.event {
            event.dispatch(&self.dispatcher, self.context.clone()).await;
        }
        Ok(())
    }
}

impl Default for App<()> {
    fn default() -> Self {
        Self::new()
    }
}
