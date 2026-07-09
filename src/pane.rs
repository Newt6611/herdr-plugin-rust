use std::path::PathBuf;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{
        PaneActionResponse, PaneCloseResponse, PaneCurrentResponse, PaneEdgesResponse,
        PaneInfoResponse, PaneLayoutResponse, PaneList, PaneProcessInfoResponse,
        PluginPaneCloseResponse, PluginPaneFocusResponse, PluginPaneOpenResponse,
    },
    socket::{env_object, insert_opt, insert_opt_bool, insert_opt_path, number},
    RuntimeHandle, RuntimeHandleError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaneSelector {
    Pane(String),
    Current,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneZoomMode {
    Toggle,
    On,
    Off,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PaneListOptions {
    pub workspace_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaneSplitOptions {
    pub pane: PaneSelector,
    pub direction: Direction,
    pub ratio: Option<f64>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub focus: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaneMoveOptions {
    pub pane_id: String,
    pub destination: PaneMoveDestination,
    pub focus: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginPanePlacement {
    Overlay,
    Split,
    Tab,
    Zoomed,
}

impl PluginPanePlacement {
    fn as_str(self) -> &'static str {
        match self {
            Self::Overlay => "overlay",
            Self::Split => "split",
            Self::Tab => "tab",
            Self::Zoomed => "zoomed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginPaneDirection {
    Right,
    Down,
}

impl PluginPaneDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Down => "down",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginPaneOpenOptions {
    pub plugin_id: String,
    pub entrypoint: String,
    pub placement: Option<PluginPanePlacement>,
    pub workspace_id: Option<String>,
    pub target_pane_id: Option<String>,
    pub direction: Option<PluginPaneDirection>,
    pub cwd: Option<PathBuf>,
    pub focus: bool,
    pub env: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaneMoveDestination {
    ExistingTab {
        tab_id: String,
        split: Direction,
        target_pane_id: Option<String>,
        ratio: Option<f64>,
    },
    NewTab {
        workspace_id: Option<String>,
        label: Option<String>,
    },
    NewWorkspace {
        label: Option<String>,
        tab_label: Option<String>,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct PaneClient<'a> {
    client: &'a HerdrClient,
}

impl<'a> PaneClient<'a> {
    pub(crate) fn new(client: &'a HerdrClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, options: PaneListOptions) -> Result<PaneList, HerdrError> {
        let mut args = vec!["pane".to_owned(), "list".to_owned()];
        if let Some(workspace_id) = options.workspace_id {
            args.push("--workspace".to_owned());
            args.push(workspace_id);
        }
        self.client.run_json_result(args).await
    }

    pub async fn current(&self, pane: PaneSelector) -> Result<PaneCurrentResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "current".to_owned()];
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn get(&self, pane_id: &str) -> Result<PaneInfoResponse, HerdrError> {
        self.client.run_json_result(["pane", "get", pane_id]).await
    }

    pub async fn layout(&self, pane: PaneSelector) -> Result<PaneLayoutResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "layout".to_owned()];
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn process_info(
        &self,
        pane: PaneSelector,
    ) -> Result<PaneProcessInfoResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "process-info".to_owned()];
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn neighbor(
        &self,
        direction: Direction,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, HerdrError> {
        let mut args = vec![
            "pane".to_owned(),
            "neighbor".to_owned(),
            "--direction".to_owned(),
            direction.as_str().to_owned(),
        ];
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn edges(&self, pane: PaneSelector) -> Result<PaneEdgesResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "edges".to_owned()];
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn focus(
        &self,
        direction: Direction,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, HerdrError> {
        let mut args = vec![
            "pane".to_owned(),
            "focus".to_owned(),
            "--direction".to_owned(),
            direction.as_str().to_owned(),
        ];
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn resize(
        &self,
        direction: Direction,
        amount: Option<f64>,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, HerdrError> {
        let mut args = vec![
            "pane".to_owned(),
            "resize".to_owned(),
            "--direction".to_owned(),
            direction.as_str().to_owned(),
        ];
        if let Some(amount) = amount {
            args.push("--amount".to_owned());
            args.push(amount.to_string());
        }
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn zoom(
        &self,
        pane: PaneSelector,
        mode: PaneZoomMode,
    ) -> Result<PaneActionResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "zoom".to_owned()];
        match pane {
            PaneSelector::Pane(pane_id) => args.push(pane_id),
            PaneSelector::Current => args.push("--current".to_owned()),
        }
        args.push(
            match mode {
                PaneZoomMode::Toggle => "--toggle",
                PaneZoomMode::On => "--on",
                PaneZoomMode::Off => "--off",
            }
            .to_owned(),
        );
        self.client.run_json_result(args).await
    }

    pub async fn rename(
        &self,
        pane_id: &str,
        label: Option<&str>,
    ) -> Result<PaneInfoResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "rename".to_owned(), pane_id.to_owned()];
        if let Some(label) = label {
            args.push(label.to_owned());
        } else {
            args.push("--clear".to_owned());
        }
        self.client.run_json_result(args).await
    }

    pub async fn split(&self, options: PaneSplitOptions) -> Result<PaneInfoResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "split".to_owned()];
        match options.pane {
            PaneSelector::Pane(pane_id) => args.push(pane_id),
            PaneSelector::Current => args.push("--current".to_owned()),
        }
        args.push("--direction".to_owned());
        args.push(options.direction.as_str().to_owned());
        if let Some(ratio) = options.ratio {
            args.push("--ratio".to_owned());
            args.push(ratio.to_string());
        }
        if let Some(cwd) = options.cwd {
            args.push("--cwd".to_owned());
            args.push(cwd.display().to_string());
        }
        for (key, value) in options.env {
            args.push("--env".to_owned());
            args.push(format!("{key}={value}"));
        }
        push_focus(&mut args, options.focus);
        self.client.run_json_result(args).await
    }

    pub async fn swap_direction(
        &self,
        direction: Direction,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, HerdrError> {
        let mut args = vec![
            "pane".to_owned(),
            "swap".to_owned(),
            "--direction".to_owned(),
            direction.as_str().to_owned(),
        ];
        push_selector(&mut args, pane);
        self.client.run_json_result(args).await
    }

    pub async fn swap_panes(
        &self,
        source_pane_id: &str,
        target_pane_id: &str,
    ) -> Result<PaneActionResponse, HerdrError> {
        self.client
            .run_json_result([
                "pane",
                "swap",
                "--source-pane",
                source_pane_id,
                "--target-pane",
                target_pane_id,
            ])
            .await
    }

    pub async fn move_pane(
        &self,
        options: PaneMoveOptions,
    ) -> Result<PaneActionResponse, HerdrError> {
        let mut args = vec!["pane".to_owned(), "move".to_owned(), options.pane_id];
        match options.destination {
            PaneMoveDestination::ExistingTab {
                tab_id,
                split,
                target_pane_id,
                ratio,
            } => {
                args.push("--tab".to_owned());
                args.push(tab_id);
                args.push("--split".to_owned());
                args.push(split.as_str().to_owned());
                if let Some(target_pane_id) = target_pane_id {
                    args.push("--target-pane".to_owned());
                    args.push(target_pane_id);
                }
                if let Some(ratio) = ratio {
                    args.push("--ratio".to_owned());
                    args.push(ratio.to_string());
                }
            }
            PaneMoveDestination::NewTab {
                workspace_id,
                label,
            } => {
                args.push("--new-tab".to_owned());
                if let Some(workspace_id) = workspace_id {
                    args.push("--workspace".to_owned());
                    args.push(workspace_id);
                }
                if let Some(label) = label {
                    args.push("--label".to_owned());
                    args.push(label);
                }
            }
            PaneMoveDestination::NewWorkspace { label, tab_label } => {
                args.push("--new-workspace".to_owned());
                if let Some(label) = label {
                    args.push("--label".to_owned());
                    args.push(label);
                }
                if let Some(tab_label) = tab_label {
                    args.push("--tab-label".to_owned());
                    args.push(tab_label);
                }
            }
        }
        push_focus(&mut args, options.focus);
        self.client.run_json_result(args).await
    }

    pub async fn close(&self, pane_id: &str) -> Result<PaneCloseResponse, HerdrError> {
        self.client
            .run_json_result(["pane", "close", pane_id])
            .await
    }

    pub async fn open_plugin_pane(
        &self,
        options: PluginPaneOpenOptions,
    ) -> Result<PluginPaneOpenResponse, HerdrError> {
        let mut args = vec![
            "plugin".to_owned(),
            "pane".to_owned(),
            "open".to_owned(),
            "--plugin".to_owned(),
            options.plugin_id,
            "--entrypoint".to_owned(),
            options.entrypoint,
        ];

        if let Some(placement) = options.placement {
            args.push("--placement".to_owned());
            args.push(placement.as_str().to_owned());
        }
        if let Some(workspace_id) = options.workspace_id {
            args.push("--workspace".to_owned());
            args.push(workspace_id);
        }
        if let Some(target_pane_id) = options.target_pane_id {
            args.push("--target-pane".to_owned());
            args.push(target_pane_id);
        }
        if let Some(direction) = options.direction {
            args.push("--direction".to_owned());
            args.push(direction.as_str().to_owned());
        }
        if let Some(cwd) = options.cwd {
            args.push("--cwd".to_owned());
            args.push(cwd.display().to_string());
        }
        for (key, value) in options.env {
            args.push("--env".to_owned());
            args.push(format!("{key}={value}"));
        }
        if options.focus {
            args.push("--focus".to_owned());
        } else {
            args.push("--no-focus".to_owned());
        }

        self.client.run_json_result(args).await
    }

    pub async fn focus_plugin_pane(
        &self,
        pane_id: &str,
    ) -> Result<PluginPaneFocusResponse, HerdrError> {
        self.client
            .run_json_result(["plugin", "pane", "focus", pane_id])
            .await
    }

    pub async fn close_plugin_pane(
        &self,
        pane_id: &str,
    ) -> Result<PluginPaneCloseResponse, HerdrError> {
        self.client
            .run_json_result(["plugin", "pane", "close", pane_id])
            .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketPaneClient<'a> {
    handle: &'a RuntimeHandle,
}

impl<'a> SocketPaneClient<'a> {
    pub(crate) fn new(handle: &'a RuntimeHandle) -> Self {
        Self { handle }
    }

    pub async fn list(&self, options: PaneListOptions) -> Result<PaneList, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        insert_opt(&mut params, "workspace_id", options.workspace_id);
        self.handle
            .request_json_result(
                "herdr-plugin:pane:list",
                "pane.list",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn current(
        &self,
        pane: PaneSelector,
    ) -> Result<PaneCurrentResponse, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        if let PaneSelector::Pane(pane_id) = pane {
            params.insert(
                "caller_pane_id".to_owned(),
                serde_json::Value::String(pane_id),
            );
        }
        self.handle
            .request_json_result(
                "herdr-plugin:pane:current",
                "pane.current",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn get(&self, pane_id: &str) -> Result<PaneInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:get",
                "pane.get",
                serde_json::json!({ "pane_id": pane_id }),
            )
            .await
    }

    pub async fn focus_pane(&self, pane_id: &str) -> Result<PaneInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:focus",
                "pane.focus",
                serde_json::json!({ "pane_id": pane_id }),
            )
            .await
    }

    pub async fn layout(
        &self,
        pane: PaneSelector,
    ) -> Result<PaneLayoutResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:layout",
                "pane.layout",
                pane_id_params(pane),
            )
            .await
    }

    pub async fn process_info(
        &self,
        pane: PaneSelector,
    ) -> Result<PaneProcessInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:process_info",
                "pane.process_info",
                pane_id_params(pane),
            )
            .await
    }

    pub async fn neighbor(
        &self,
        direction: Direction,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, RuntimeHandleError> {
        let mut params = pane_id_map(pane);
        params.insert(
            "direction".to_owned(),
            serde_json::Value::String(direction.as_str().to_owned()),
        );
        self.handle
            .request_json_result(
                "herdr-plugin:pane:neighbor",
                "pane.neighbor",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn edges(&self, pane: PaneSelector) -> Result<PaneEdgesResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:edges",
                "pane.edges",
                pane_id_params(pane),
            )
            .await
    }

    pub async fn focus(
        &self,
        direction: Direction,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, RuntimeHandleError> {
        let mut params = pane_id_map(pane);
        params.insert(
            "direction".to_owned(),
            serde_json::Value::String(direction.as_str().to_owned()),
        );
        self.handle
            .request_json_result(
                "herdr-plugin:pane:focus_direction",
                "pane.focus_direction",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn resize(
        &self,
        direction: Direction,
        amount: Option<f64>,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, RuntimeHandleError> {
        let mut params = pane_id_map(pane);
        params.insert(
            "direction".to_owned(),
            serde_json::Value::String(direction.as_str().to_owned()),
        );
        if let Some(amount) = amount {
            params.insert("amount".to_owned(), number(amount));
        }
        self.handle
            .request_json_result(
                "herdr-plugin:pane:resize",
                "pane.resize",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn zoom(
        &self,
        pane: PaneSelector,
        mode: PaneZoomMode,
    ) -> Result<PaneActionResponse, RuntimeHandleError> {
        let mut params = pane_id_map(pane);
        params.insert(
            "mode".to_owned(),
            serde_json::Value::String(mode.as_str().to_owned()),
        );
        self.handle
            .request_json_result(
                "herdr-plugin:pane:zoom",
                "pane.zoom",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn rename(
        &self,
        pane_id: &str,
        label: Option<&str>,
    ) -> Result<PaneInfoResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:rename",
                "pane.rename",
                serde_json::json!({ "pane_id": pane_id, "label": label }),
            )
            .await
    }

    pub async fn split(
        &self,
        options: PaneSplitOptions,
    ) -> Result<PaneInfoResponse, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        if let PaneSelector::Pane(pane_id) = options.pane {
            params.insert(
                "target_pane_id".to_owned(),
                serde_json::Value::String(pane_id),
            );
        }
        params.insert(
            "direction".to_owned(),
            serde_json::Value::String(options.direction.as_str().to_owned()),
        );
        if let Some(ratio) = options.ratio {
            params.insert("ratio".to_owned(), number(ratio));
        }
        insert_opt_path(&mut params, "cwd", options.cwd);
        if !options.env.is_empty() {
            params.insert("env".to_owned(), env_object(options.env));
        }
        insert_opt_bool(&mut params, "focus", options.focus);
        self.handle
            .request_json_result(
                "herdr-plugin:pane:split",
                "pane.split",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn swap_direction(
        &self,
        direction: Direction,
        pane: PaneSelector,
    ) -> Result<PaneActionResponse, RuntimeHandleError> {
        let mut params = pane_id_map(pane);
        params.insert(
            "direction".to_owned(),
            serde_json::Value::String(direction.as_str().to_owned()),
        );
        self.handle
            .request_json_result(
                "herdr-plugin:pane:swap",
                "pane.swap",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn swap_panes(
        &self,
        source_pane_id: &str,
        target_pane_id: &str,
    ) -> Result<PaneActionResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:swap",
                "pane.swap",
                serde_json::json!({
                    "source_pane_id": source_pane_id,
                    "target_pane_id": target_pane_id
                }),
            )
            .await
    }

    pub async fn move_pane(
        &self,
        options: PaneMoveOptions,
    ) -> Result<PaneActionResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:move",
                "pane.move",
                serde_json::json!({
                    "pane_id": options.pane_id,
                    "destination": socket_move_destination(options.destination),
                    "focus": options.focus.unwrap_or(false)
                }),
            )
            .await
    }

    pub async fn send_text(
        &self,
        pane_id: &str,
        text: &str,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:send_text",
                "pane.send_text",
                serde_json::json!({ "pane_id": pane_id, "text": text }),
            )
            .await
    }

    pub async fn send_keys(
        &self,
        pane_id: &str,
        keys: Vec<String>,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:send_keys",
                "pane.send_keys",
                serde_json::json!({ "pane_id": pane_id, "keys": keys }),
            )
            .await
    }

    pub async fn send_input(
        &self,
        pane_id: &str,
        text: impl Into<String>,
        keys: Vec<String>,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:send_input",
                "pane.send_input",
                serde_json::json!({ "pane_id": pane_id, "text": text.into(), "keys": keys }),
            )
            .await
    }

    pub async fn read(
        &self,
        pane_id: &str,
        options: crate::AgentReadOptions,
    ) -> Result<crate::AgentReadResponse, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        params.insert(
            "pane_id".to_owned(),
            serde_json::Value::String(pane_id.to_owned()),
        );
        if let Some(source) = options.source {
            params.insert(
                "source".to_owned(),
                serde_json::Value::String(source.as_str().to_owned()),
            );
        }
        if let Some(lines) = options.lines {
            params.insert("lines".to_owned(), serde_json::Value::Number(lines.into()));
        }
        if let Some(format) = options.format {
            params.insert(
                "format".to_owned(),
                serde_json::Value::String(format.as_str().to_owned()),
            );
        }
        params.insert(
            "strip_ansi".to_owned(),
            serde_json::Value::Bool(!options.ansi),
        );
        self.handle
            .request_json_result(
                "herdr-plugin:pane:read",
                "pane.read",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn report_agent(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:report_agent",
                "pane.report_agent",
                params,
            )
            .await
    }

    pub async fn report_agent_session(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:report_agent_session",
                "pane.report_agent_session",
                params,
            )
            .await
    }

    pub async fn report_metadata(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:report_metadata",
                "pane.report_metadata",
                params,
            )
            .await
    }

    pub async fn clear_agent_authority(
        &self,
        pane_id: &str,
        source: Option<&str>,
        seq: Option<u64>,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:clear_agent_authority",
                "pane.clear_agent_authority",
                serde_json::json!({ "pane_id": pane_id, "source": source, "seq": seq }),
            )
            .await
    }

    pub async fn release_agent(
        &self,
        pane_id: &str,
        source: &str,
        agent: &str,
        seq: Option<u64>,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:release_agent",
                "pane.release_agent",
                serde_json::json!({
                    "pane_id": pane_id,
                    "source": source,
                    "agent": agent,
                    "seq": seq
                }),
            )
            .await
    }

    pub async fn wait_for_output(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:wait_for_output",
                "pane.wait_for_output",
                params,
            )
            .await
    }

    pub async fn close(&self, pane_id: &str) -> Result<PaneCloseResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:pane:close",
                "pane.close",
                serde_json::json!({ "pane_id": pane_id }),
            )
            .await
    }

    pub async fn open_plugin_pane(
        &self,
        options: PluginPaneOpenOptions,
    ) -> Result<PluginPaneOpenResponse, RuntimeHandleError> {
        let mut params = serde_json::Map::new();
        params.insert(
            "plugin_id".to_owned(),
            serde_json::Value::String(options.plugin_id),
        );
        params.insert(
            "entrypoint".to_owned(),
            serde_json::Value::String(options.entrypoint),
        );
        if let Some(placement) = options.placement {
            params.insert(
                "placement".to_owned(),
                serde_json::Value::String(placement.as_str().to_owned()),
            );
        }
        insert_opt(&mut params, "workspace_id", options.workspace_id);
        insert_opt(&mut params, "target_pane_id", options.target_pane_id);
        if let Some(direction) = options.direction {
            params.insert(
                "direction".to_owned(),
                serde_json::Value::String(direction.as_str().to_owned()),
            );
        }
        insert_opt_path(&mut params, "cwd", options.cwd);
        params.insert("focus".to_owned(), serde_json::Value::Bool(options.focus));
        if !options.env.is_empty() {
            params.insert("env".to_owned(), env_object(options.env));
        }
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:pane:open",
                "plugin.pane.open",
                serde_json::Value::Object(params),
            )
            .await
    }

    pub async fn focus_plugin_pane(
        &self,
        pane_id: &str,
    ) -> Result<PluginPaneFocusResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:pane:focus",
                "plugin.pane.focus",
                serde_json::json!({ "pane_id": pane_id }),
            )
            .await
    }

    pub async fn close_plugin_pane(
        &self,
        pane_id: &str,
    ) -> Result<PluginPaneCloseResponse, RuntimeHandleError> {
        self.handle
            .request_json_result(
                "herdr-plugin:plugin:pane:close",
                "plugin.pane.close",
                serde_json::json!({ "pane_id": pane_id }),
            )
            .await
    }
}

impl RuntimeHandle {
    pub fn pane(&self) -> SocketPaneClient<'_> {
        SocketPaneClient::new(self)
    }
}

pub(crate) fn push_selector(args: &mut Vec<String>, pane: PaneSelector) {
    match pane {
        PaneSelector::Pane(pane_id) => {
            args.push("--pane".to_owned());
            args.push(pane_id);
        }
        PaneSelector::Current => args.push("--current".to_owned()),
    }
}

pub(crate) fn push_focus(args: &mut Vec<String>, focus: Option<bool>) {
    match focus {
        Some(true) => args.push("--focus".to_owned()),
        Some(false) => args.push("--no-focus".to_owned()),
        None => {}
    }
}

fn pane_id_params(pane: PaneSelector) -> serde_json::Value {
    serde_json::Value::Object(pane_id_map(pane))
}

fn pane_id_map(pane: PaneSelector) -> serde_json::Map<String, serde_json::Value> {
    let mut params = serde_json::Map::new();
    if let PaneSelector::Pane(pane_id) = pane {
        params.insert("pane_id".to_owned(), serde_json::Value::String(pane_id));
    }
    params
}

fn socket_move_destination(destination: PaneMoveDestination) -> serde_json::Value {
    match destination {
        PaneMoveDestination::ExistingTab {
            tab_id,
            split,
            target_pane_id,
            ratio,
        } => {
            let mut value = serde_json::Map::new();
            value.insert(
                "type".to_owned(),
                serde_json::Value::String("tab".to_owned()),
            );
            value.insert("tab_id".to_owned(), serde_json::Value::String(tab_id));
            value.insert(
                "split".to_owned(),
                serde_json::Value::String(split.as_str().to_owned()),
            );
            if let Some(target_pane_id) = target_pane_id {
                value.insert(
                    "target_pane_id".to_owned(),
                    serde_json::Value::String(target_pane_id),
                );
            }
            if let Some(ratio) = ratio {
                value.insert("ratio".to_owned(), number(ratio));
            }
            serde_json::Value::Object(value)
        }
        PaneMoveDestination::NewTab {
            workspace_id,
            label,
        } => {
            let mut value = serde_json::Map::new();
            value.insert(
                "type".to_owned(),
                serde_json::Value::String("new_tab".to_owned()),
            );
            insert_opt(&mut value, "workspace_id", workspace_id);
            insert_opt(&mut value, "label", label);
            serde_json::Value::Object(value)
        }
        PaneMoveDestination::NewWorkspace { label, tab_label } => {
            let mut value = serde_json::Map::new();
            value.insert(
                "type".to_owned(),
                serde_json::Value::String("new_workspace".to_owned()),
            );
            insert_opt(&mut value, "label", label);
            insert_opt(&mut value, "tab_label", tab_label);
            serde_json::Value::Object(value)
        }
    }
}

impl PaneZoomMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Toggle => "toggle",
            Self::On => "on",
            Self::Off => "off",
        }
    }
}
