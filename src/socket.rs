use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::{RuntimeHandle, RuntimeHandleError};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ServerLiveHandoffOptions {
    pub import_exe: Option<String>,
    pub expected_protocol: Option<u32>,
    pub expected_version: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationSound {
    None,
    Done,
    Request,
}

impl NotificationSound {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Done => "done",
            Self::Request => "request",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NotificationShowOptions {
    pub title: String,
    pub body: Option<String>,
    pub position: Option<String>,
    pub sound: Option<NotificationSound>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayoutExportOptions {
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutApplyOptions {
    pub workspace_id: Option<String>,
    pub tab_id: Option<String>,
    pub tab_label: Option<String>,
    pub focus: bool,
    pub root: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSetSplitRatioOptions {
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
    pub path: Vec<bool>,
    pub ratio: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrationTarget {
    pub target: String,
}

#[derive(Clone, Copy, Debug)]
pub struct SocketServerClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketServerClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn ping(&self) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result("herdr-plugin:ping", "ping", empty_params())
            .await
    }

    pub async fn stop(&self) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result("herdr-plugin:server:stop", "server.stop", empty_params())
            .await
    }

    pub async fn live_handoff(
        &self,
        options: ServerLiveHandoffOptions,
    ) -> Result<Value, RuntimeHandleError> {
        let mut params = Map::new();
        insert_opt(&mut params, "import_exe", options.import_exe);
        insert_opt_u32(&mut params, "expected_protocol", options.expected_protocol);
        insert_opt(&mut params, "expected_version", options.expected_version);
        self.handle
            .request_json_result(
                "herdr-plugin:server:live_handoff",
                "server.live_handoff",
                Value::Object(params),
            )
            .await
    }

    pub async fn reload_config(&self) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:server:reload_config",
                "server.reload_config",
                empty_params(),
            )
            .await
    }

    pub async fn agent_manifests(&self) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:server:agent_manifests",
                "server.agent_manifests",
                empty_params(),
            )
            .await
    }

    pub async fn reload_agent_manifests(&self) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:server:reload_agent_manifests",
                "server.reload_agent_manifests",
                empty_params(),
            )
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketNotificationClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketNotificationClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn show(
        &self,
        options: NotificationShowOptions,
    ) -> Result<Value, RuntimeHandleError> {
        let mut params = Map::new();
        params.insert("title".to_owned(), Value::String(options.title));
        insert_opt(&mut params, "body", options.body);
        insert_opt(&mut params, "position", options.position);
        if let Some(sound) = options.sound {
            params.insert("sound".to_owned(), Value::String(sound.as_str().to_owned()));
        }
        self.handle
            .request_json_result(
                "herdr-plugin:notification:show",
                "notification.show",
                Value::Object(params),
            )
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketClientControl<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketClientControl<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn set_window_title(&self, title: &str) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:client:window_title:set",
                "client.window_title.set",
                serde_json::json!({ "title": title }),
            )
            .await
    }

    pub async fn clear_window_title(&self) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:client:window_title:clear",
                "client.window_title.clear",
                empty_params(),
            )
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketSessionClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketSessionClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn snapshot(&self) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:session:snapshot",
                "session.snapshot",
                empty_params(),
            )
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketLayoutClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketLayoutClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn export(&self, options: LayoutExportOptions) -> Result<Value, RuntimeHandleError> {
        let mut params = Map::new();
        insert_opt(&mut params, "tab_id", options.tab_id);
        insert_opt(&mut params, "pane_id", options.pane_id);
        self.handle
            .request_json_result(
                "herdr-plugin:layout:export",
                "layout.export",
                Value::Object(params),
            )
            .await
    }

    pub async fn apply(&self, options: LayoutApplyOptions) -> Result<Value, RuntimeHandleError> {
        let mut params = Map::new();
        insert_opt(&mut params, "workspace_id", options.workspace_id);
        insert_opt(&mut params, "tab_id", options.tab_id);
        insert_opt(&mut params, "tab_label", options.tab_label);
        params.insert("focus".to_owned(), Value::Bool(options.focus));
        params.insert("root".to_owned(), options.root);
        self.handle
            .request_json_result(
                "herdr-plugin:layout:apply",
                "layout.apply",
                Value::Object(params),
            )
            .await
    }

    pub async fn set_split_ratio(
        &self,
        options: LayoutSetSplitRatioOptions,
    ) -> Result<Value, RuntimeHandleError> {
        let mut params = Map::new();
        insert_opt(&mut params, "tab_id", options.tab_id);
        insert_opt(&mut params, "pane_id", options.pane_id);
        params.insert(
            "path".to_owned(),
            Value::Array(options.path.into_iter().map(Value::Bool).collect()),
        );
        params.insert("ratio".to_owned(), number(options.ratio));
        self.handle
            .request_json_result(
                "herdr-plugin:layout:set_split_ratio",
                "layout.set_split_ratio",
                Value::Object(params),
            )
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketEventsClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketEventsClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn subscribe(&self, params: Value) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result("herdr-plugin:events:subscribe", "events.subscribe", params)
            .await
    }

    pub async fn wait(&self, params: Value) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result("herdr-plugin:events:wait", "events.wait", params)
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketIntegrationClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketIntegrationClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn install(&self, target: IntegrationTarget) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:integration:install",
                "integration.install",
                serde_json::json!({ "target": target.target }),
            )
            .await
    }

    pub async fn uninstall(&self, target: IntegrationTarget) -> Result<Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:integration:uninstall",
                "integration.uninstall",
                serde_json::json!({ "target": target.target }),
            )
            .await
    }
}

impl RuntimeHandle {
    pub fn server(&self) -> SocketServerClient<'_> {
        SocketServerClient::new(self)
    }

    pub fn notification(&self) -> SocketNotificationClient<'_> {
        SocketNotificationClient::new(self)
    }

    pub fn client_control(&self) -> SocketClientControl<'_> {
        SocketClientControl::new(self)
    }

    pub fn client(&self) -> SocketClientControl<'_> {
        SocketClientControl::new(self)
    }

    pub fn socket_session(&self) -> SocketSessionClient<'_> {
        SocketSessionClient::new(self)
    }

    pub fn session(&self) -> SocketSessionClient<'_> {
        SocketSessionClient::new(self)
    }

    pub fn layout(&self) -> SocketLayoutClient<'_> {
        SocketLayoutClient::new(self)
    }

    pub fn events(&self) -> SocketEventsClient<'_> {
        SocketEventsClient::new(self)
    }

    pub fn integration(&self) -> SocketIntegrationClient<'_> {
        SocketIntegrationClient::new(self)
    }
}

pub(crate) fn empty_params() -> Value {
    Value::Object(Map::new())
}

pub(crate) fn env_object(env: Vec<(String, String)>) -> Value {
    Value::Object(
        env.into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect(),
    )
}

pub(crate) fn insert_opt(params: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), Value::String(value));
    }
}

pub(crate) fn insert_opt_path(params: &mut Map<String, Value>, key: &str, value: Option<PathBuf>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), Value::String(value.display().to_string()));
    }
}

pub(crate) fn insert_opt_bool(params: &mut Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), Value::Bool(value));
    }
}

pub(crate) fn insert_opt_u32(params: &mut Map<String, Value>, key: &str, value: Option<u32>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), Value::Number(value.into()));
    }
}

pub(crate) fn number(value: f64) -> Value {
    Value::Number(serde_json::Number::from_f64(value).expect("finite JSON number"))
}
