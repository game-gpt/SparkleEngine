//! Studio 宿主：`GameHost` + `UiRuntime`。

use spark_core::Vec2;
use spark_input::Key;
use spark_renderer::{DrawList, FrameCtx, GameHost};
use spark_widget::{Insets, UiCommand, UiFrame, UiRuntime};

use crate::shell::{self, CMD_GALLERY, CMD_PLAY};

pub struct StudioApp {
    ui: UiRuntime,
    project: String,
    status: String,
    exit: bool,
    mounted: bool,
}

impl StudioApp {
    pub fn new(project: impl Into<String>) -> Self {
        let project = project.into();
        Self {
            ui: UiRuntime::new(),
            status: format!("已打开 · {project}"),
            project,
            exit: false,
            mounted: false,
        }
    }

    fn ensure_mounted(&mut self) {
        if self.mounted {
            return;
        }
        self.ui.mount_scene(shell::build_shell(&self.project));
        self.mounted = true;
    }

    fn handle_commands(&mut self) {
        for cmd in self.ui.drain_commands() {
            match cmd {
                UiCommand::Custom(CMD_PLAY) => {
                    self.status = "Play：尚未接运行时预览（占位）".into();
                    tracing::info!(status = %self.status, "命令");
                }
                UiCommand::Custom(CMD_GALLERY) => {
                    self.status = "Widget Gallery：面板尚未接入（占位）".into();
                    tracing::info!(status = %self.status, "命令");
                }
                UiCommand::Custom(id) => {
                    self.status = format!("命令 Custom({id})");
                    tracing::info!(status = %self.status, "命令");
                }
                other => {
                    tracing::debug!(?other, "未处理 UI 命令");
                }
            }
        }
    }
}

impl GameHost for StudioApp {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        if frame.input.key_pressed(Key::Escape) {
            self.exit = true;
            return;
        }

        self.ensure_mounted();

        let ui_frame = UiFrame {
            dt: frame.dt,
            screen_size: Vec2::new(frame.screen_w, frame.screen_h),
            dpi_scale: 1.0,
            ui_scale: 1.0,
            safe_area: Insets::default(),
            input: frame.input,
        };

        self.ui.begin_frame(&ui_frame);
        self.ui.dispatch_input(&ui_frame);
        self.ui.update(frame.dt);
        self.ui.layout(&ui_frame);
        self.handle_commands();
        self.ui.end_frame();
    }

    fn draw(&mut self, draw: &mut DrawList) {
        self.ui.paint(draw);
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
