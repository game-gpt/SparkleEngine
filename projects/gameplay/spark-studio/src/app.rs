//! Studio 宿主：`GameHost` + `UiRuntime`；Play 时嵌入示例对局。

use spark_core::Vec2;
use spark_input::Key;
use spark_renderer::{DrawList, FrameCtx, GameHost};
use spark_widget::{Insets, UiCommand, UiFrame, UiRuntime};

use crate::play::PlaySession;
use crate::project::{ProjectInfo, list_asset_entries};
use crate::shell;
use crate::state::{
    BottomTab, CMD_BOTTOM_CONSOLE, CMD_BOTTOM_PROBLEMS, CMD_BOTTOM_PROJECT, CMD_PAUSE, CMD_PLAY,
    CMD_STEP, CMD_STOP, CMD_TAB_GAME, CMD_TAB_SCENE, CMD_TAB_SCRIPT, CMD_TOOL_HAND, CMD_TOOL_MOVE,
    CMD_TOOL_ROTATE, CMD_TOOL_SCALE, CMD_WINDOW_GALLERY, CenterTab, EditorState, PlayMode, Tool,
    default_selected, parse_select_cmd,
};

pub struct StudioApp {
    ui: UiRuntime,
    project: ProjectInfo,
    assets: Vec<String>,
    state: EditorState,
    play: Option<PlaySession>,
    exit: bool,
    mounted: bool,
    dirty_ui: bool,
}

impl StudioApp {
    pub fn new(project: ProjectInfo) -> Self {
        let assets = list_asset_entries(&project.root);
        let mut state = EditorState::default();
        state.selected = default_selected(project.kind);
        let inferred = if project.kind_inferred {
            "（推断）"
        } else {
            ""
        };
        state.status = format!(
            "已打开 {} · kind={}{}",
            project.name,
            project.kind.as_str(),
            inferred
        );
        Self {
            ui: UiRuntime::new(),
            project,
            assets,
            state,
            play: None,
            exit: false,
            mounted: false,
            dirty_ui: true,
        }
    }

    /// `--play`：跳过编辑器，直接进入对局。
    pub fn with_immediate_play(mut self) -> Self {
        match PlaySession::start(&self.project) {
            Ok(session) => {
                self.play = Some(session);
                self.state.play = PlayMode::Play;
                self.state.center = CenterTab::Game;
                self.state.status = format!("Play：{}", self.project.name);
            }
            Err(e) => {
                self.state.status = format!("无法 Play：{e}");
                self.state.bottom = BottomTab::Console;
            }
        }
        self
    }

    fn remount(&mut self) {
        self.ui
            .mount_scene(shell::build_shell(&self.project, &self.state, &self.assets));
        self.mounted = true;
        self.dirty_ui = false;
    }

    fn start_play(&mut self) {
        match PlaySession::start(&self.project) {
            Ok(session) => {
                self.play = Some(session);
                self.state.play = PlayMode::Play;
                self.state.center = CenterTab::Game;
                self.state.status = format!("Play：正在运行 {}", self.project.name);
                self.dirty_ui = true;
            }
            Err(e) => {
                self.state.status = format!("Play 失败：{e}");
                self.state.bottom = BottomTab::Console;
                self.dirty_ui = true;
            }
        }
    }

    fn stop_play(&mut self) {
        self.play = None;
        self.state.play = PlayMode::Edit;
        self.state.center = CenterTab::Scene;
        self.state.status = "已停止 · 返回 Edit".into();
        self.dirty_ui = true;
    }

    fn handle_commands(&mut self) {
        let cmds: Vec<_> = self.ui.drain_commands().collect();
        for cmd in cmds {
            match cmd {
                UiCommand::Custom(CMD_PLAY) => {
                    self.start_play();
                }
                UiCommand::Custom(CMD_PAUSE) => {
                    if self.state.play == PlayMode::Play {
                        self.state.play = PlayMode::Paused;
                        self.state.status = "Paused".into();
                        self.dirty_ui = true;
                    }
                }
                UiCommand::Custom(CMD_STEP) => {
                    self.state.status = "Step：单帧（占位）".into();
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_STOP) => {
                    self.stop_play();
                }
                UiCommand::Custom(CMD_TOOL_HAND) => {
                    self.state.tool = Tool::Hand;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_TOOL_MOVE) => {
                    self.state.tool = Tool::Move;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_TOOL_ROTATE) => {
                    self.state.tool = Tool::Rotate;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_TOOL_SCALE) => {
                    self.state.tool = Tool::Scale;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_TAB_SCENE) => {
                    self.state.center = CenterTab::Scene;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_TAB_GAME) => {
                    self.state.center = CenterTab::Game;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_TAB_SCRIPT) => {
                    self.state.center = CenterTab::Script;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_BOTTOM_PROJECT) => {
                    self.state.bottom = BottomTab::Project;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_BOTTOM_CONSOLE) => {
                    self.state.bottom = BottomTab::Console;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_BOTTOM_PROBLEMS) => {
                    self.state.bottom = BottomTab::Problems;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_WINDOW_GALLERY) => {
                    self.state.status = "Window → Widget Gallery（调试工具，不占主导航）".into();
                    self.state.bottom = BottomTab::Console;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(id) => {
                    if let Some(eid) = parse_select_cmd(id) {
                        self.state.selected = eid;
                        self.dirty_ui = true;
                    } else {
                        self.state.status = format!("命令 {id}");
                        self.dirty_ui = true;
                    }
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
        if self.play.is_some() {
            if self.state.play == PlayMode::Paused {
                return;
            }
            let stop = self.play.as_mut().map(|p| p.update(frame)).unwrap_or(false);
            if stop || frame.input.key_pressed(Key::Escape) {
                self.stop_play();
            }
            return;
        }

        if frame.input.key_pressed(Key::Escape) {
            self.exit = true;
            return;
        }

        if !self.mounted || self.dirty_ui {
            self.remount();
        }

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
        if self.dirty_ui {
            self.remount();
            self.ui.layout(&ui_frame);
        }
        self.ui.end_frame();
    }

    fn draw(&mut self, draw: &mut DrawList) {
        if let Some(play) = self.play.as_mut() {
            play.draw(draw);
            return;
        }
        self.ui.paint(draw);
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
