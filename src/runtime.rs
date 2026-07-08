use std::{future::Future, pin::Pin};

use tokio::sync::mpsc;

use crate::{
    context::Context,
    dispatcher::EventDispatcher,
    event_source::{EnvEventSource, EventSourceOutput, RuntimeEvent},
    load_config, HerdrClient, RuntimeError, SetupError, SetupHandler, TeardownHandler,
};
use serde::de::DeserializeOwned;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub type RuntimeFuture = Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + 'static>>;

pub trait Runtime<State, Config>: Send + 'static
where
    State: Send + Sync + 'static,
    Config: DeserializeOwned + Default + Send + Sync + 'static,
{
    fn run(self, app: RuntimeApp<State, Config>) -> RuntimeFuture;
}

#[derive(Debug, Default)]
pub struct OneShotRuntime;

impl OneShotRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl<State, Config> Runtime<State, Config> for OneShotRuntime
where
    State: Send + Sync + 'static,
    Config: DeserializeOwned + Default + Send + Sync + 'static,
{
    fn run(self, mut app: RuntimeApp<State, Config>) -> RuntimeFuture {
        Box::pin(async move {
            let output = EnvEventSource::from_env()?;
            app.initialize(output)?;

            if let Err(source) = app.run_setup().await {
                return app.return_error(RuntimeError::Setup { source }).await;
            }

            app.dispatch_next_event().await;

            if let Err(source) = app.run_teardown().await {
                return app.return_error(RuntimeError::Teardown { source }).await;
            }

            Ok(())
        })
    }
}

pub struct RuntimeApp<State, Config> {
    client: Option<Arc<HerdrClient>>,
    state: Arc<Mutex<State>>,
    config: Option<Config>,
    config_path: Option<PathBuf>,
    herdr_bin_path_override: Option<PathBuf>,
    context: Option<Context<State, Config>>,
    event: Option<RuntimeEvent>,
    dispatcher: EventDispatcher<Context<State, Config>>,
    setup_handlers: Vec<SetupHandler<State, Config>>,
    teardown_handlers: Vec<TeardownHandler<State, Config>>,
    error_handlers: Vec<crate::ErrorHandler<State, Config>>,
}

impl<State, Config> RuntimeApp<State, Config>
where
    State: Send + Sync + 'static,
    Config: DeserializeOwned + Default + Send + Sync + 'static,
{
    pub fn initialize(&mut self, mut output: EventSourceOutput) -> Result<(), RuntimeError> {
        if let Some(path) = self.herdr_bin_path_override.as_ref() {
            output.env.bin_path = Some(path.clone());
        }

        let client = self.client.clone().unwrap_or_else(|| {
            output
                .env
                .bin_path
                .as_ref()
                .map(|path| Arc::new(HerdrClient::with_binary(path.clone())))
                .unwrap_or_else(|| Arc::new(HerdrClient::new()))
        });

        let config = Arc::new(match self.config_path.as_ref() {
            Some(path) => load_config::<Config>(&output.env, path)
                .map_err(|source| RuntimeError::Config { source })?,
            None => self
                .config
                .take()
                .expect("runtime app initialized more than once"),
        });

        self.context = Some(Context::with_env_state_and_config(
            client,
            output.env,
            self.state.clone(),
            config,
        ));
        self.event = output.event;
        Ok(())
    }

    pub fn context(&self) -> Context<State, Config> {
        self.context
            .as_ref()
            .expect("runtime app has not been initialized")
            .clone()
    }

    pub async fn run_setup(&self) -> Result<(), SetupError> {
        let context = self.context();
        for handler in &self.setup_handlers {
            handler(context.clone()).await?;
        }
        Ok(())
    }

    pub async fn dispatch_next_event(&mut self) {
        let context = self.context();
        if let Some(event) = self.event.take() {
            event.dispatch(&self.dispatcher, context).await;
        }
    }

    pub async fn run_teardown(&self) -> Result<(), SetupError> {
        let context = self.context();
        for handler in &self.teardown_handlers {
            handler(context.clone()).await?;
        }
        Ok(())
    }

    pub async fn return_error(&self, error: RuntimeError) -> Result<(), RuntimeError> {
        let message = error.to_string();
        let context = self.context();
        for handler in &self.error_handlers {
            handler(context.clone(), message.clone()).await;
        }
        Err(error)
    }
}

impl<State, Config> RuntimeApp<State, Config> {
    pub(crate) fn new(
        client: Option<Arc<HerdrClient>>,
        state: Arc<Mutex<State>>,
        config: Config,
        config_path: Option<PathBuf>,
        herdr_bin_path_override: Option<PathBuf>,
        dispatcher: EventDispatcher<Context<State, Config>>,
        setup_handlers: Vec<SetupHandler<State, Config>>,
        teardown_handlers: Vec<TeardownHandler<State, Config>>,
        error_handlers: Vec<crate::ErrorHandler<State, Config>>,
    ) -> Self {
        Self {
            client,
            state,
            config: Some(config),
            config_path,
            herdr_bin_path_override,
            context: None,
            event: None,
            dispatcher,
            setup_handlers,
            teardown_handlers,
            error_handlers,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum RuntimeCommand {
    Stop,
}

#[derive(Clone, Debug)]
pub struct RuntimeHandle {
    command_tx: mpsc::Sender<RuntimeCommand>,
}

impl RuntimeHandle {
    #[allow(dead_code)]
    pub(crate) fn new(command_tx: mpsc::Sender<RuntimeCommand>) -> Self {
        Self { command_tx }
    }

    pub async fn stop(&self) -> Result<(), RuntimeHandleError> {
        self.command_tx
            .send(RuntimeCommand::Stop)
            .await
            .map_err(|_| RuntimeHandleError::RuntimeStopped)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeHandleError {
    #[error("runtime is no longer accepting commands")]
    RuntimeStopped,
}

#[allow(dead_code)]
pub(crate) fn runtime_command_channel(
    buffer: usize,
) -> (RuntimeHandle, mpsc::Receiver<RuntimeCommand>) {
    let (command_tx, command_rx) = mpsc::channel(buffer);
    (RuntimeHandle::new(command_tx), command_rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runtime_handle_stop_sends_stop_command() {
        let (handle, mut command_rx) = runtime_command_channel(1);

        handle.stop().await.unwrap();

        assert_eq!(command_rx.recv().await, Some(RuntimeCommand::Stop));
    }
}
