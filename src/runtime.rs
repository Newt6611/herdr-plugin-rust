use std::{
    future::Future,
    io::{BufRead, BufReader, Write},
    path::Path,
    pin::Pin,
    sync::{Arc as StdArc, Mutex as StdMutex},
};

use tokio::sync::{mpsc, oneshot};

use crate::error::HerdrCommandErrorBody;
use crate::{
    context::Context,
    dispatcher::EventDispatcher,
    event_source::{EnvEventSource, EventSourceOutput},
    load_config, HerdrClient, RuntimeError, SetupError, SetupHandler, TeardownHandler,
};
use crate::{
    events::{EventEnvelope, EventKind},
    RuntimeEvent,
};
use serde::{de::DeserializeOwned, Deserialize};
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

pub struct SocketRuntime {
    handle: RuntimeHandle,
    command_rx: mpsc::Receiver<RuntimeCommand>,
    subscriptions: Vec<SocketSubscription>,
}

impl SocketRuntime {
    const COMMAND_BUFFER: usize = 32;

    pub fn new() -> Self {
        let (handle, command_rx) = runtime_command_channel(Self::COMMAND_BUFFER);
        Self {
            handle,
            command_rx,
            subscriptions: default_socket_subscriptions(),
        }
    }

    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    pub fn subscribe(mut self, events: impl IntoIterator<Item = EventKind>) -> Self {
        self.subscriptions = events.into_iter().map(SocketSubscription::Event).collect();
        self
    }

    pub fn subscribe_all(mut self) -> Self {
        self.subscriptions = default_socket_subscriptions();
        self
    }

    pub fn without_subscriptions(mut self) -> Self {
        self.subscriptions.clear();
        self
    }
}

impl Default for SocketRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl<State, Config> Runtime<State, Config> for SocketRuntime
where
    State: Send + Sync + 'static,
    Config: DeserializeOwned + Default + Send + Sync + 'static,
{
    fn run(mut self, mut app: RuntimeApp<State, Config>) -> RuntimeFuture {
        Box::pin(async move {
            let output = EnvEventSource::from_env()?;
            app.initialize(output)?;
            app.set_socket_handle(self.handle.clone());

            if let Err(source) = app.run_setup().await {
                return app.return_error(RuntimeError::Setup { source }).await;
            }

            let socket_path = app
                .context()
                .env()
                .socket_path
                .clone()
                .ok_or(RuntimeError::MissingSocketPath)?;
            self.handle.set_socket_path(socket_path.clone());
            let mut event_stream =
                SocketEventStream::connect(&socket_path, self.subscriptions.clone()).await?;

            loop {
                tokio::select! {
                    command = self.command_rx.recv() => {
                        match command {
                            Some(RuntimeCommand::Stop) | None => break,
                        }
                    }
                    event = event_stream.next_event() => {
                        match event {
                            Ok(Some(event)) => app.dispatch_event(event).await,
                            Ok(None) => break,
                            Err(error) => return app.return_error(error).await,
                        }
                    }
                }
            }

            if let Err(source) = app.run_teardown().await {
                return app.return_error(RuntimeError::Teardown { source }).await;
            }

            Ok(())
        })
    }
}

struct SocketEventStream {
    event_rx: mpsc::Receiver<Result<RuntimeEvent, RuntimeError>>,
}

impl SocketEventStream {
    async fn connect(
        path: &Path,
        subscriptions: Vec<SocketSubscription>,
    ) -> Result<Self, RuntimeError> {
        let path = path.to_path_buf();
        let (event_tx, event_rx) = mpsc::channel(32);
        let (ready_tx, ready_rx) = oneshot::channel();

        tokio::task::spawn_blocking(move || {
            run_socket_event_reader(path, subscriptions, event_tx, ready_tx);
        });

        ready_rx
            .await
            .map_err(|_| RuntimeError::SocketSubscription {
                message: "socket event reader exited before startup completed".to_owned(),
            })??;

        Ok(Self { event_rx })
    }

    async fn next_event(&mut self) -> Result<Option<RuntimeEvent>, RuntimeError> {
        match self.event_rx.recv().await {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SocketSubscription {
    Event(EventKind),
}

impl SocketSubscription {
    fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Event(event) => serde_json::json!({ "type": event.dot_name() }),
        }
    }
}

fn default_socket_subscriptions() -> Vec<SocketSubscription> {
    [
        EventKind::WorkspaceCreated,
        EventKind::WorkspaceUpdated,
        EventKind::WorkspaceClosed,
        EventKind::WorkspaceRenamed,
        EventKind::WorkspaceMoved,
        EventKind::WorkspaceFocused,
        EventKind::TabCreated,
        EventKind::TabClosed,
        EventKind::TabRenamed,
        EventKind::TabMoved,
        EventKind::TabFocused,
        EventKind::PaneCreated,
        EventKind::PaneClosed,
        EventKind::PaneFocused,
        EventKind::PaneMoved,
        EventKind::PaneExited,
        EventKind::PaneAgentDetected,
    ]
    .into_iter()
    .map(SocketSubscription::Event)
    .collect()
}

fn socket_subscribe_request(subscriptions: &[SocketSubscription]) -> String {
    serde_json::json!({
        "id": "herdr-plugin:events",
        "method": "events.subscribe",
        "params": {
            "subscriptions": subscriptions.iter().map(SocketSubscription::to_json).collect::<Vec<_>>()
        }
    })
    .to_string()
}

fn run_socket_event_reader(
    path: PathBuf,
    subscriptions: Vec<SocketSubscription>,
    event_tx: mpsc::Sender<Result<RuntimeEvent, RuntimeError>>,
    ready_tx: oneshot::Sender<Result<(), RuntimeError>>,
) {
    let mut stream = match connect_local_stream(&path).map_err(|source| RuntimeError::SocketIo {
        path: path.clone(),
        source,
    }) {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    if let Err(error) = write_json_line(&mut stream, socket_subscribe_request(&subscriptions))
        .map_err(|source| RuntimeError::SocketIo {
            path: path.clone(),
            source,
        })
    {
        let _ = ready_tx.send(Err(error));
        return;
    }

    let mut reader = BufReader::new(stream);
    let ack = match read_line(&mut reader).map_err(|source| RuntimeError::SocketIo {
        path: path.clone(),
        source,
    }) {
        Ok(ack) => ack,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    if let Err(error) = validate_subscription_ack(ack) {
        let _ = ready_tx.send(Err(error));
        return;
    }
    let _ = ready_tx.send(Ok(()));

    loop {
        let line = match read_line(&mut reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(source) => {
                let _ = event_tx.blocking_send(Err(RuntimeError::SocketIo {
                    path: path.clone(),
                    source,
                }));
                break;
            }
        };

        let event = serde_json::from_str::<EventEnvelope>(&line)
            .map(RuntimeEvent::from)
            .map_err(|source| RuntimeError::InvalidSocketJson { json: line, source });
        if event_tx.blocking_send(event).is_err() {
            break;
        }
    }
}

fn validate_subscription_ack(ack: Option<String>) -> Result<(), RuntimeError> {
    let Some(ack) = ack else {
        return Err(RuntimeError::SocketSubscription {
            message: "socket closed before subscription ack".to_owned(),
        });
    };

    let value: serde_json::Value =
        serde_json::from_str(&ack).map_err(|source| RuntimeError::InvalidSocketJson {
            json: ack.clone(),
            source,
        })?;

    if value
        .get("result")
        .and_then(|result| result.get("type"))
        .and_then(|kind| kind.as_str())
        == Some("subscription_started")
    {
        return Ok(());
    }

    if let Some(message) = value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
    {
        return Err(RuntimeError::SocketSubscription {
            message: message.to_owned(),
        });
    }

    Err(RuntimeError::SocketSubscription {
        message: format!("unexpected subscription ack: {value}"),
    })
}

async fn send_socket_request_inner(
    path: &Path,
    request: serde_json::Value,
) -> Result<serde_json::Value, RuntimeError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || send_socket_request_blocking(path, request))
        .await
        .map_err(|_| RuntimeError::SocketSubscription {
            message: "socket request task panicked".to_owned(),
        })?
}

fn send_socket_request_blocking(
    path: PathBuf,
    request: serde_json::Value,
) -> Result<serde_json::Value, RuntimeError> {
    let mut stream = connect_local_stream(&path).map_err(|source| RuntimeError::SocketIo {
        path: path.clone(),
        source,
    })?;
    write_json_line(&mut stream, request.to_string()).map_err(|source| RuntimeError::SocketIo {
        path: path.clone(),
        source,
    })?;

    let mut reader = BufReader::new(stream);
    let response = read_line(&mut reader)
        .map_err(|source| RuntimeError::SocketIo {
            path: path.clone(),
            source,
        })?
        .ok_or_else(|| RuntimeError::SocketSubscription {
            message: "socket closed before command response".to_owned(),
        })?;

    serde_json::from_str(&response).map_err(|source| RuntimeError::InvalidSocketJson {
        json: response,
        source,
    })
}

type LocalStream = interprocess::local_socket::Stream;

fn connect_local_stream(path: &Path) -> std::io::Result<LocalStream> {
    #[cfg(unix)]
    {
        use interprocess::local_socket::{prelude::*, GenericFilePath};

        let name = path.to_fs_name::<GenericFilePath>()?;
        LocalStream::connect(name)
    }

    #[cfg(windows)]
    {
        use interprocess::local_socket::{prelude::*, GenericNamespaced};

        let name = path.to_string_lossy().to_string();
        let name = name.to_ns_name::<GenericNamespaced>()?;
        LocalStream::connect(name)
    }
}

fn write_json_line(stream: &mut LocalStream, json: String) -> std::io::Result<()> {
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

fn read_line(reader: &mut BufReader<LocalStream>) -> std::io::Result<Option<String>> {
    let mut line = String::new();
    let read = reader.read_line(&mut line)?;
    if read == 0 {
        return Ok(None);
    }
    Ok(Some(line))
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

    pub fn set_socket_handle(&mut self, handle: RuntimeHandle) {
        let context = self
            .context
            .take()
            .expect("runtime app has not been initialized");
        self.context = Some(context.with_socket(handle));
    }

    pub async fn run_setup(&self) -> Result<(), SetupError> {
        let context = self.context();
        for handler in &self.setup_handlers {
            handler(context.clone()).await?;
        }
        Ok(())
    }

    pub async fn dispatch_next_event(&mut self) {
        if let Some(event) = self.event.take() {
            self.dispatch_event(event).await;
        }
    }

    pub async fn dispatch_event(&self, event: RuntimeEvent) {
        let context = self.context();
        event.dispatch(&self.dispatcher, context).await;
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
    socket_path: StdArc<StdMutex<Option<PathBuf>>>,
}

impl RuntimeHandle {
    #[allow(dead_code)]
    pub(crate) fn new(command_tx: mpsc::Sender<RuntimeCommand>) -> Self {
        Self {
            command_tx,
            socket_path: StdArc::new(StdMutex::new(None)),
        }
    }

    pub(crate) fn set_socket_path(&self, socket_path: PathBuf) {
        *self
            .socket_path
            .lock()
            .expect("runtime handle socket path mutex poisoned") = Some(socket_path);
    }

    pub async fn stop(&self) -> Result<(), RuntimeHandleError> {
        self.command_tx
            .send(RuntimeCommand::Stop)
            .await
            .map_err(|_| RuntimeHandleError::RuntimeStopped)
    }

    pub async fn request_json(
        &self,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        let socket_path = self
            .socket_path
            .lock()
            .expect("runtime handle socket path mutex poisoned")
            .clone()
            .ok_or(RuntimeHandleError::SocketUnavailable)?;
        send_socket_request_inner(&socket_path, request)
            .await
            .map_err(RuntimeHandleError::CommandFailed)
    }

    pub(crate) async fn request_json_result<T>(
        &self,
        id: impl Into<String>,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, RuntimeHandleError>
    where
        T: DeserializeOwned,
    {
        let response = self
            .request_json(serde_json::json!({
                "id": id.into(),
                "method": method,
                "params": params,
            }))
            .await?;
        parse_socket_response(response)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeHandleError {
    #[error("runtime is no longer accepting commands")]
    RuntimeStopped,
    #[error("socket runtime has not initialized a socket path")]
    SocketUnavailable,
    #[error("Herdr command failed")]
    CommandFailed(#[source] RuntimeError),
    #[error("{0}")]
    ApiError(#[source] crate::HerdrCommandError),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SocketResponse<T> {
    Success { result: T },
    Error(HerdrCommandErrorBody),
}

fn parse_socket_response<T>(response: serde_json::Value) -> Result<T, RuntimeHandleError>
where
    T: DeserializeOwned,
{
    match serde_json::from_value(response).map_err(|source| {
        RuntimeHandleError::CommandFailed(RuntimeError::InvalidSocketJson {
            json: "<socket response>".to_owned(),
            source,
        })
    })? {
        SocketResponse::Success { result } => Ok(result),
        SocketResponse::Error(error) => Err(RuntimeHandleError::ApiError(error.error)),
    }
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
    use std::sync::{Arc, Mutex};

    use super::*;

    static SOCKET_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn runtime_handle_stop_sends_stop_command() {
        let (handle, mut command_rx) = runtime_command_channel(1);

        handle.stop().await.unwrap();

        assert!(matches!(
            command_rx.recv().await,
            Some(RuntimeCommand::Stop)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_runtime_blocks_until_handle_stops_it() {
        let _env_lock = SOCKET_ENV_LOCK.lock().await;
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::UnixListener,
        };

        let socket_path = std::env::temp_dir().join(format!(
            "herdr-plugin-runtime-block-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        let previous_socket_path = std::env::var_os("HERDR_SOCKET_PATH");
        std::env::set_var("HERDR_SOCKET_PATH", &socket_path);

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            reader.read_line(&mut request).await.unwrap();

            let mut stream = reader.into_inner();
            stream
                .write_all(
                    br#"{"id":"herdr-plugin:events","result":{"type":"subscription_started"}}"#,
                )
                .await
                .unwrap();
            stream.write_all(b"\n").await.unwrap();
            std::future::pending::<()>().await;
        });

        let runtime = SocketRuntime::new();
        let handle = runtime.handle();
        let calls = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let (setup_tx, setup_rx) = tokio::sync::oneshot::channel();
        let setup_tx = Arc::new(Mutex::new(Some(setup_tx)));

        let app = crate::App::builder()
            .runtime(runtime)
            .build()
            .unwrap()
            .setup({
                let calls = calls.clone();
                let setup_tx = setup_tx.clone();
                move |_ctx: Context| {
                    let calls = calls.clone();
                    let setup_tx = setup_tx.clone();
                    async move {
                        calls.lock().unwrap().push("setup");
                        if let Some(setup_tx) = setup_tx.lock().unwrap().take() {
                            let _ = setup_tx.send(());
                        }
                        Ok(())
                    }
                }
            })
            .teardown({
                let calls = calls.clone();
                move |_ctx: Context| {
                    let calls = calls.clone();
                    async move {
                        calls.lock().unwrap().push("teardown");
                        Ok(())
                    }
                }
            });

        let run_task = tokio::spawn(app.run());
        setup_rx.await.unwrap();

        assert_eq!(*calls.lock().unwrap(), ["setup"]);

        handle.stop().await.unwrap();
        run_task.await.unwrap().unwrap();

        assert_eq!(*calls.lock().unwrap(), ["setup", "teardown"]);

        server.abort();
        let _ = std::fs::remove_file(&socket_path);
        match previous_socket_path {
            Some(value) => std::env::set_var("HERDR_SOCKET_PATH", value),
            None => std::env::remove_var("HERDR_SOCKET_PATH"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_runtime_subscribes_to_socket_events_and_dispatches_them() {
        let _env_lock = SOCKET_ENV_LOCK.lock().await;
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::UnixListener,
        };

        let socket_path = std::env::temp_dir().join(format!(
            "herdr-plugin-runtime-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        let previous_socket_path = std::env::var_os("HERDR_SOCKET_PATH");
        std::env::set_var("HERDR_SOCKET_PATH", &socket_path);

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            reader.read_line(&mut request).await.unwrap();

            let request: serde_json::Value = serde_json::from_str(&request).unwrap();
            assert_eq!(request["method"], "events.subscribe");

            let mut stream = reader.into_inner();
            stream
                .write_all(
                    br#"{"id":"herdr-plugin:events","result":{"type":"subscription_started"}}"#,
                )
                .await
                .unwrap();
            stream.write_all(b"\n").await.unwrap();
            stream
                .write_all(
                    br#"{"event":"tab_renamed","data":{"type":"tab_renamed","tab_id":"w1:t1","workspace_id":"w1","label":"socket"}}"#,
                )
                .await
                .unwrap();
            stream.write_all(b"\n").await.unwrap();
            std::future::pending::<()>().await;
        });

        let runtime = SocketRuntime::new();
        let handle = runtime.handle();
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let (setup_tx, setup_rx) = tokio::sync::oneshot::channel();
        let setup_tx = Arc::new(Mutex::new(Some(setup_tx)));

        let app = crate::App::builder()
            .runtime(runtime)
            .build()
            .unwrap()
            .setup({
                let setup_tx = setup_tx.clone();
                move |_ctx: Context| {
                    let setup_tx = setup_tx.clone();
                    async move {
                        if let Some(setup_tx) = setup_tx.lock().unwrap().take() {
                            let _ = setup_tx.send(());
                        }
                        Ok(())
                    }
                }
            })
            .on_event::<crate::TabRenamed>({
                let calls = calls.clone();
                move |_ctx: Context, event: crate::TabRenamed| {
                    let calls = calls.clone();
                    async move {
                        calls.lock().unwrap().push(format!("event:{}", event.label));
                    }
                }
            })
            .teardown({
                let calls = calls.clone();
                move |_ctx: Context| {
                    let calls = calls.clone();
                    async move {
                        calls.lock().unwrap().push("teardown".to_owned());
                        Ok(())
                    }
                }
            });

        let run_task = tokio::spawn(app.run());
        setup_rx.await.unwrap();

        while calls.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }

        assert_eq!(*calls.lock().unwrap(), ["event:socket"]);

        handle.stop().await.unwrap();
        run_task.await.unwrap().unwrap();

        assert_eq!(*calls.lock().unwrap(), ["event:socket", "teardown"]);

        server.abort();
        let _ = std::fs::remove_file(&socket_path);
        match previous_socket_path {
            Some(value) => std::env::set_var("HERDR_SOCKET_PATH", value),
            None => std::env::remove_var("HERDR_SOCKET_PATH"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_runtime_uses_configured_event_subscriptions() {
        let _env_lock = SOCKET_ENV_LOCK.lock().await;
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::UnixListener,
        };

        let socket_path = std::env::temp_dir().join(format!(
            "herdr-plugin-runtime-subscriptions-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        let previous_socket_path = std::env::var_os("HERDR_SOCKET_PATH");
        std::env::set_var("HERDR_SOCKET_PATH", &socket_path);

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            reader.read_line(&mut request).await.unwrap();

            let request: serde_json::Value = serde_json::from_str(&request).unwrap();
            assert_eq!(
                request["params"]["subscriptions"],
                serde_json::json!([{ "type": "tab.renamed" }])
            );

            let mut stream = reader.into_inner();
            stream
                .write_all(
                    br#"{"id":"herdr-plugin:events","result":{"type":"subscription_started"}}"#,
                )
                .await
                .unwrap();
            stream.write_all(b"\n").await.unwrap();
            std::future::pending::<()>().await;
        });

        let runtime = SocketRuntime::new().subscribe([EventKind::TabRenamed]);
        let handle = runtime.handle();
        let (setup_tx, setup_rx) = tokio::sync::oneshot::channel();
        let setup_tx = Arc::new(Mutex::new(Some(setup_tx)));

        let app = crate::App::builder()
            .runtime(runtime)
            .build()
            .unwrap()
            .setup({
                let setup_tx = setup_tx.clone();
                move |_ctx: Context| {
                    let setup_tx = setup_tx.clone();
                    async move {
                        if let Some(setup_tx) = setup_tx.lock().unwrap().take() {
                            let _ = setup_tx.send(());
                        }
                        Ok(())
                    }
                }
            });

        let run_task = tokio::spawn(app.run());
        setup_rx.await.unwrap();

        handle.stop().await.unwrap();
        run_task.await.unwrap().unwrap();

        server.abort();
        let _ = std::fs::remove_file(&socket_path);
        match previous_socket_path {
            Some(value) => std::env::set_var("HERDR_SOCKET_PATH", value),
            None => std::env::remove_var("HERDR_SOCKET_PATH"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_runtime_handle_sends_json_request_to_herdr() {
        let _env_lock = SOCKET_ENV_LOCK.lock().await;
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::UnixListener,
        };

        let socket_path = std::env::temp_dir().join(format!(
            "herdr-plugin-runtime-command-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        let previous_socket_path = std::env::var_os("HERDR_SOCKET_PATH");
        std::env::set_var("HERDR_SOCKET_PATH", &socket_path);

        let server = tokio::spawn(async move {
            let (subscription, _) = listener.accept().await.unwrap();
            let mut subscription_reader = BufReader::new(subscription);
            let mut subscribe_request = String::new();
            subscription_reader
                .read_line(&mut subscribe_request)
                .await
                .unwrap();
            let mut subscription = subscription_reader.into_inner();
            subscription
                .write_all(
                    br#"{"id":"herdr-plugin:events","result":{"type":"subscription_started"}}"#,
                )
                .await
                .unwrap();
            subscription.write_all(b"\n").await.unwrap();

            let (command, _) = listener.accept().await.unwrap();
            let mut command_reader = BufReader::new(command);
            let mut command_request = String::new();
            command_reader
                .read_line(&mut command_request)
                .await
                .unwrap();
            let command_request: serde_json::Value =
                serde_json::from_str(&command_request).unwrap();
            assert_eq!(command_request["method"], "ping");

            let mut command = command_reader.into_inner();
            command
                .write_all(br#"{"id":"req_ping","result":{"type":"pong","version":"test","protocol":1,"capabilities":null}}"#)
                .await
                .unwrap();
            command.write_all(b"\n").await.unwrap();
            std::future::pending::<()>().await;
        });

        let runtime = SocketRuntime::new();
        let handle = runtime.handle();
        let (setup_tx, setup_rx) = tokio::sync::oneshot::channel();
        let setup_tx = Arc::new(Mutex::new(Some(setup_tx)));

        let app = crate::App::builder()
            .runtime(runtime)
            .build()
            .unwrap()
            .setup({
                let setup_tx = setup_tx.clone();
                move |_ctx: Context| {
                    let setup_tx = setup_tx.clone();
                    async move {
                        if let Some(setup_tx) = setup_tx.lock().unwrap().take() {
                            let _ = setup_tx.send(());
                        }
                        Ok(())
                    }
                }
            });

        let run_task = tokio::spawn(app.run());
        setup_rx.await.unwrap();

        let response = handle
            .request_json(serde_json::json!({
                "id": "req_ping",
                "method": "ping",
                "params": {}
            }))
            .await
            .unwrap();
        assert_eq!(response["result"]["type"], "pong");

        handle.stop().await.unwrap();
        run_task.await.unwrap().unwrap();

        server.abort();
        let _ = std::fs::remove_file(&socket_path);
        match previous_socket_path {
            Some(value) => std::env::set_var("HERDR_SOCKET_PATH", value),
            None => std::env::remove_var("HERDR_SOCKET_PATH"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_runtime_handle_workspace_create_uses_typed_api() {
        let _env_lock = SOCKET_ENV_LOCK.lock().await;
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::UnixListener,
        };

        let socket_path = std::env::temp_dir().join(format!(
            "herdr-plugin-runtime-workspace-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        let previous_socket_path = std::env::var_os("HERDR_SOCKET_PATH");
        std::env::set_var("HERDR_SOCKET_PATH", &socket_path);

        let server = tokio::spawn(async move {
            let (subscription, _) = listener.accept().await.unwrap();
            let mut subscription_reader = BufReader::new(subscription);
            let mut subscribe_request = String::new();
            subscription_reader
                .read_line(&mut subscribe_request)
                .await
                .unwrap();
            let mut subscription = subscription_reader.into_inner();
            subscription
                .write_all(
                    br#"{"id":"herdr-plugin:events","result":{"type":"subscription_started"}}"#,
                )
                .await
                .unwrap();
            subscription.write_all(b"\n").await.unwrap();

            let (command, _) = listener.accept().await.unwrap();
            let mut command_reader = BufReader::new(command);
            let mut command_request = String::new();
            command_reader
                .read_line(&mut command_request)
                .await
                .unwrap();
            let command_request: serde_json::Value =
                serde_json::from_str(&command_request).unwrap();
            assert_eq!(command_request["method"], "workspace.create");
            assert_eq!(command_request["params"]["cwd"], "/tmp");
            assert_eq!(command_request["params"]["label"], "probe");
            assert_eq!(command_request["params"]["env"]["KEY"], "VALUE");
            assert_eq!(command_request["params"]["focus"], false);

            let mut command = command_reader.into_inner();
            command
                .write_all(br#"{"id":"herdr-plugin:workspace:create","result":{"root_pane":{"agent_status":"unknown","cwd":"/tmp","focused":false,"foreground_cwd":"/tmp","pane_id":"wW:p1","revision":0,"tab_id":"wW:t1","terminal_id":"term_1","workspace_id":"wW"},"tab":{"agent_status":"unknown","focused":false,"label":"1","number":1,"pane_count":1,"tab_id":"wW:t1","workspace_id":"wW"},"type":"workspace_created","workspace":{"active_tab_id":"wW:t1","agent_status":"unknown","focused":false,"label":"probe","number":5,"pane_count":1,"tab_count":1,"workspace_id":"wW"}}}"#)
                .await
                .unwrap();
            command.write_all(b"\n").await.unwrap();
            std::future::pending::<()>().await;
        });

        let runtime = SocketRuntime::new();
        let handle = runtime.handle();
        let (setup_tx, setup_rx) = tokio::sync::oneshot::channel();
        let setup_tx = Arc::new(Mutex::new(Some(setup_tx)));

        let app = crate::App::builder()
            .runtime(runtime)
            .build()
            .unwrap()
            .setup({
                let setup_tx = setup_tx.clone();
                move |_ctx: Context| {
                    let setup_tx = setup_tx.clone();
                    async move {
                        if let Some(setup_tx) = setup_tx.lock().unwrap().take() {
                            let _ = setup_tx.send(());
                        }
                        Ok(())
                    }
                }
            });

        let run_task = tokio::spawn(app.run());
        setup_rx.await.unwrap();

        let created = handle
            .workspace()
            .create(crate::WorkspaceCreateOptions {
                cwd: Some(PathBuf::from("/tmp")),
                label: Some("probe".to_owned()),
                env: vec![("KEY".to_owned(), "VALUE".to_owned())],
                focus: Some(false),
            })
            .await
            .unwrap();
        assert_eq!(created.workspace.workspace_id, "wW");

        handle.stop().await.unwrap();
        run_task.await.unwrap().unwrap();

        server.abort();
        let _ = std::fs::remove_file(&socket_path);
        match previous_socket_path {
            Some(value) => std::env::set_var("HERDR_SOCKET_PATH", value),
            None => std::env::remove_var("HERDR_SOCKET_PATH"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_runtime_handle_exposes_protocol_method_groups() {
        let _env_lock = SOCKET_ENV_LOCK.lock().await;
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::UnixListener,
        };

        let socket_path = std::env::temp_dir().join(format!(
            "herdr-plugin-runtime-all-methods-test-{}.sock",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();

        let expected = vec![
            ("ping", serde_json::json!({})),
            ("server.stop", serde_json::json!({})),
            ("server.reload_config", serde_json::json!({})),
            ("server.agent_manifests", serde_json::json!({})),
            ("server.reload_agent_manifests", serde_json::json!({})),
            (
                "server.live_handoff",
                serde_json::json!({"expected_protocol":1,"expected_version":"test"}),
            ),
            (
                "notification.show",
                serde_json::json!({"title":"done","body":"ok","sound":"done"}),
            ),
            (
                "client.window_title.set",
                serde_json::json!({"title":"Herdr"}),
            ),
            ("client.window_title.clear", serde_json::json!({})),
            ("session.snapshot", serde_json::json!({})),
            (
                "workspace.move",
                serde_json::json!({"workspace_id":"w1","insert_index":2}),
            ),
            (
                "tab.move",
                serde_json::json!({"tab_id":"w1:t1","insert_index":3}),
            ),
            (
                "agent.send",
                serde_json::json!({"target":"w1:p1","text":"hello"}),
            ),
            (
                "pane.send_text",
                serde_json::json!({"pane_id":"w1:p1","text":"hello"}),
            ),
            (
                "pane.send_keys",
                serde_json::json!({"pane_id":"w1:p1","keys":["enter"]}),
            ),
            (
                "pane.send_input",
                serde_json::json!({"pane_id":"w1:p1","text":"hi","keys":["enter"]}),
            ),
            (
                "pane.report_agent",
                serde_json::json!({"pane_id":"w1:p1","source":"test","agent":"bot","state":"working"}),
            ),
            (
                "pane.report_agent_session",
                serde_json::json!({"pane_id":"w1:p1","source":"test","agent":"bot","agent_session_id":"s1"}),
            ),
            (
                "pane.report_metadata",
                serde_json::json!({"pane_id":"w1:p1","source":"test","title":"work"}),
            ),
            (
                "pane.clear_agent_authority",
                serde_json::json!({"pane_id":"w1:p1","source":"test","seq":1}),
            ),
            (
                "pane.release_agent",
                serde_json::json!({"pane_id":"w1:p1","source":"test","agent":"bot","seq":1}),
            ),
            (
                "pane.wait_for_output",
                serde_json::json!({"pane_id":"w1:p1","pattern":"ready"}),
            ),
            ("layout.export", serde_json::json!({"tab_id":"w1:t1"})),
            (
                "layout.apply",
                serde_json::json!({"focus":false,"root":{"type":"pane"}}),
            ),
            (
                "layout.set_split_ratio",
                serde_json::json!({"path":[true,false],"ratio":0.6}),
            ),
            (
                "events.subscribe",
                serde_json::json!({"subscriptions":[{"type":"tab.focused"}]}),
            ),
            (
                "events.wait",
                serde_json::json!({"match_event":{"event":"tab_focused","tab_id":"w1:t1"}}),
            ),
            ("integration.install", serde_json::json!({"target":"codex"})),
            (
                "integration.uninstall",
                serde_json::json!({"target":"codex"}),
            ),
            (
                "plugin.link",
                serde_json::json!({"path":"/plugin","enabled":true}),
            ),
            (
                "plugin.unlink",
                serde_json::json!({"plugin_id":"example.tool"}),
            ),
            (
                "plugin.action.list",
                serde_json::json!({"plugin_id":"example.tool"}),
            ),
            (
                "plugin.action.invoke",
                serde_json::json!({"action_id":"example.tool.run","context":{"invocation_source":"test"}}),
            ),
            (
                "plugin.log.list",
                serde_json::json!({"plugin_id":"example.tool","limit":5}),
            ),
        ];

        let server_expected = expected.clone();
        let server = tokio::spawn(async move {
            for (index, (method, params)) in server_expected.into_iter().enumerate() {
                let (command, _) = listener.accept().await.unwrap();
                let mut reader = BufReader::new(command);
                let mut request = String::new();
                reader.read_line(&mut request).await.unwrap();
                let request: serde_json::Value = serde_json::from_str(&request).unwrap();
                assert_eq!(request["method"], method);
                assert_eq!(request["params"], params);

                let mut command = reader.into_inner();
                command
                    .write_all(
                        format!(
                            r#"{{"id":"response-{index}","result":{{"type":"ok","index":{index}}}}}"#
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                command.write_all(b"\n").await.unwrap();
            }
        });

        let (handle, _rx) = runtime_command_channel(1);
        handle.set_socket_path(socket_path.clone());

        handle.server().ping().await.unwrap();
        handle.server().stop().await.unwrap();
        handle.server().reload_config().await.unwrap();
        handle.server().agent_manifests().await.unwrap();
        handle.server().reload_agent_manifests().await.unwrap();
        handle
            .server()
            .live_handoff(crate::ServerLiveHandoffOptions {
                import_exe: None,
                expected_protocol: Some(1),
                expected_version: Some("test".to_owned()),
            })
            .await
            .unwrap();
        handle
            .notification()
            .show(crate::NotificationShowOptions {
                title: "done".to_owned(),
                body: Some("ok".to_owned()),
                position: None,
                sound: Some(crate::NotificationSound::Done),
            })
            .await
            .unwrap();
        handle.client().set_window_title("Herdr").await.unwrap();
        handle.client().clear_window_title().await.unwrap();
        handle.session().snapshot().await.unwrap();
        handle.workspace().move_workspace("w1", 2).await.unwrap();
        handle.tab().move_tab("w1:t1", 3).await.unwrap();
        handle.agent().send("w1:p1", "hello").await.unwrap();
        handle.pane().send_text("w1:p1", "hello").await.unwrap();
        handle
            .pane()
            .send_keys("w1:p1", vec!["enter".to_owned()])
            .await
            .unwrap();
        handle
            .pane()
            .send_input("w1:p1", "hi", vec!["enter".to_owned()])
            .await
            .unwrap();
        handle
            .pane()
            .report_agent(
                serde_json::json!({"pane_id":"w1:p1","source":"test","agent":"bot","state":"working"}),
            )
            .await
            .unwrap();
        handle
            .pane()
            .report_agent_session(
                serde_json::json!({"pane_id":"w1:p1","source":"test","agent":"bot","agent_session_id":"s1"}),
            )
            .await
            .unwrap();
        handle
            .pane()
            .report_metadata(serde_json::json!({"pane_id":"w1:p1","source":"test","title":"work"}))
            .await
            .unwrap();
        handle
            .pane()
            .clear_agent_authority("w1:p1", Some("test"), Some(1))
            .await
            .unwrap();
        handle
            .pane()
            .release_agent("w1:p1", "test", "bot", Some(1))
            .await
            .unwrap();
        handle
            .pane()
            .wait_for_output(serde_json::json!({"pane_id":"w1:p1","pattern":"ready"}))
            .await
            .unwrap();
        handle
            .layout()
            .export(crate::LayoutExportOptions {
                tab_id: Some("w1:t1".to_owned()),
                pane_id: None,
            })
            .await
            .unwrap();
        handle
            .layout()
            .apply(crate::LayoutApplyOptions {
                workspace_id: None,
                tab_id: None,
                tab_label: None,
                focus: false,
                root: serde_json::json!({"type":"pane"}),
            })
            .await
            .unwrap();
        handle
            .layout()
            .set_split_ratio(crate::LayoutSetSplitRatioOptions {
                tab_id: None,
                pane_id: None,
                path: vec![true, false],
                ratio: 0.6,
            })
            .await
            .unwrap();
        handle
            .events()
            .subscribe(serde_json::json!({"subscriptions":[{"type":"tab.focused"}]}))
            .await
            .unwrap();
        handle
            .events()
            .wait(serde_json::json!({"match_event":{"event":"tab_focused","tab_id":"w1:t1"}}))
            .await
            .unwrap();
        handle
            .integration()
            .install(crate::IntegrationTarget {
                target: "codex".to_owned(),
            })
            .await
            .unwrap();
        handle
            .integration()
            .uninstall(crate::IntegrationTarget {
                target: "codex".to_owned(),
            })
            .await
            .unwrap();
        handle
            .plugin()
            .link(crate::PluginLinkOptions {
                path: "/plugin".to_owned(),
                enabled: true,
                source: None,
            })
            .await
            .unwrap();
        handle.plugin().unlink("example.tool").await.unwrap();
        handle
            .plugin()
            .action_list(crate::PluginActionListOptions {
                plugin_id: Some("example.tool".to_owned()),
            })
            .await
            .unwrap();
        handle
            .plugin()
            .action_invoke(crate::PluginActionInvokeOptions {
                action_id: "example.tool.run".to_owned(),
                plugin_id: None,
                context: Some(serde_json::json!({"invocation_source":"test"})),
            })
            .await
            .unwrap();
        handle
            .plugin()
            .log_list(crate::PluginLogListOptions {
                plugin_id: Some("example.tool".to_owned()),
                limit: Some(5),
            })
            .await
            .unwrap();

        server.await.unwrap();
        let _ = std::fs::remove_file(&socket_path);
    }
}
