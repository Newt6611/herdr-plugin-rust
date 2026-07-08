use std::path::PathBuf;

use crate::{
    client::HerdrClient,
    error::HerdrError,
    models::{
        PaneActionResponse, PaneCloseResponse, PaneCurrentResponse, PaneEdgesResponse,
        PaneInfoResponse, PaneLayoutResponse, PaneList, PaneProcessInfoResponse,
        PluginPaneCloseResponse, PluginPaneFocusResponse, PluginPaneOpenResponse,
    },
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
