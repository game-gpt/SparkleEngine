//! Studio 窗口泵：`WindowPump2d` + `UiRuntime`；Play 时保留编辑器壳，Game 页签显示对局。
//!
//! 不变式：编辑器壳与对局会话可并存；仅在 `--play` 浸入模式下从不挂载壳，
//! Esc / Stop 才回到 Edit（浸入模式则退出进程级 Play）。

use spark_input::{Input, Key, MouseBtn};
use spark_renderer::{DrawList, FrameCtx, WindowPump2d};
use spark_types::Vec2;
use spark_widget::{Insets, Theme, UiCommand, UiFrame, UiRuntime};

use crate::{
    layout::{
        LayoutPreset, SplitterAxis, SplitterDrag, apply_splitter_drag, center_viewport_rect, double_click_splitter, hit_splitter,
        load_dock_layout, save_dock_layout,
    },
    play::PlaySession,
    project::{ProjectInfo, list_asset_entries},
    state::{
        BottomTab, CMD_BOTTOM_CONSOLE, CMD_BOTTOM_PROBLEMS, CMD_BOTTOM_PROJECT, CMD_LAYOUT_DEBUG, CMD_LAYOUT_DEFAULT, CMD_LAYOUT_SCRIPT,
        CMD_PAUSE, CMD_PLAY, CMD_STEP, CMD_STOP, CMD_TAB_GAME, CMD_TAB_SCENE, CMD_TAB_SCRIPT, CMD_TOOL_HAND, CMD_TOOL_MOVE, CMD_TOOL_ROTATE,
        CMD_TOOL_SCALE, CMD_WINDOW_GALLERY, CenterTab,
        EditorState, PlayMode, Tool, TransformState, default_selected, entity_by_id, is_transform_field_key, parse_asset_cmd,
        parse_select_cmd, pick_entity_at_world,
    },
    ui,
    viewport::{paint_scene_viewport, screen_to_world},
};

/// Studio 应用宿主：持有 Widget 运行时、项目元数据与可选 Play 会话。
///
/// 实现 [`WindowPump2d`]：每帧先处理 UI 命令，再按 `PlayMode` 推进对局。
pub struct StudioApp {
    ui: UiRuntime,
    project: ProjectInfo,
    assets: Vec<String>,
    state: EditorState,
    play: Option<PlaySession>,
    splitter_drag: Option<SplitterDrag>,
    splitter_last_click: Option<(SplitterAxis, f64)>,
    viewport_pan: Option<(f32, f32)>,
    entity_drag: Option<(f32, f32)>,
    last_focus_key: Option<String>,
    screen_w: f32,
    screen_h: f32,
    exit: bool,
    mounted: bool,
    dirty_ui: bool,
}

impl StudioApp {
    /// 打开项目：扫描 `assets/`、按 kind 选默认 Hierarchy 行，状态栏写打开信息。
    pub fn new(project: ProjectInfo) -> Self {
        let assets = list_asset_entries(&project.root);
        let mut state = EditorState::default();
        state.selected = default_selected(project.kind);
        if let Some(dock) = load_dock_layout(&project.root) {
            state.dock = dock;
        }
        let inferred = if project.kind_inferred { "（推断）" } else { "" };
        state.status = format!("已打开 {} · kind={}{}", project.name, project.kind.as_str(), inferred);
        let mut ui = UiRuntime::new();
        ui.theme = Theme::editor_dark();
        Self {
            ui,
            project,
            assets,
            state,
            play: None,
            splitter_drag: None,
            splitter_last_click: None,
            viewport_pan: None,
            entity_drag: None,
            last_focus_key: None,
            screen_w: 1280.0,
            screen_h: 720.0,
            exit: false,
            mounted: false,
            dirty_ui: true,
        }
    }

    /// `--play`：跳过编辑器壳，直接进入对局全屏（仍可用 Esc 退出进程级 play）。
    pub fn with_immediate_play(mut self) -> Self {
        match PlaySession::start(&self.project) {
            Ok(session) => {
                let label = session.label();
                self.play = Some(session);
                self.state.play = PlayMode::Play;
                self.state.center = CenterTab::Game;
                self.state.status = format!("运行中：{label}");
            }
            Err(e) => {
                self.state.status = format!("无法运行：{e}");
                self.state.bottom = BottomTab::Console;
            }
        }
        self
    }

    fn remount(&mut self) {
        let shell = ui::build_shell(&self.project, &self.state, &self.assets);
        if self.mounted {
            self.ui.reconcile_scene(shell);
        }
        else {
            self.ui.mount_scene(shell);
        }
        self.mounted = true;
        self.dirty_ui = false;
    }

    fn persist_dock(&self) {
        if let Err(err) = save_dock_layout(&self.project.root, &self.state.dock) {
            tracing::warn!(%err, "保存停靠布局失败");
        }
    }

    fn apply_layout_preset(&mut self, preset: LayoutPreset) {
        self.state.layout_preset = preset;
        self.state.dock = preset.apply_to(self.state.dock);
        self.state.status = format!("已应用{}布局", preset.label());
        self.dirty_ui = true;
        self.persist_dock();
    }

    fn start_play(&mut self) {
        match PlaySession::start(&self.project) {
            Ok(session) => {
                let label = session.label();
                self.play = Some(session);
                self.state.play = PlayMode::Play;
                self.state.center = CenterTab::Game;
                self.state.status = format!("运行中：{label}");
                self.dirty_ui = true;
            }
            Err(e) => {
                self.state.status = format!("运行失败：{e}");
                self.state.bottom = BottomTab::Console;
                self.dirty_ui = true;
            }
        }
    }

    fn stop_play(&mut self) {
        self.play = None;
        self.state.play = PlayMode::Edit;
        self.state.center = CenterTab::Scene;
        self.state.status = "已停止，返回编辑模式".into();
        self.dirty_ui = true;
    }

    fn apply_selection(&mut self, entity_id: u64) {
        self.state.selected = entity_id;
        self.state.transform = TransformState::from_entity(entity_id);
        self.dirty_ui = true;
    }

    fn sync_inspector_transform(&mut self) {
        if let Some(root) = self.ui.scene_root() {
            if TransformState::read_from_tree(&self.ui.tree, root, &mut self.state.transform) {
                self.state.status = "已应用 Transform 修改".into();
            }
        }
    }

    fn push_transform_to_inspector(&mut self) {
        if let Some(root) = self.ui.scene_root() {
            self.state.transform.write_to_tree(&mut self.ui.tree, root);
            self.ui.invalidate_paint();
        }
    }

    fn focused_widget_key(&self) -> Option<String> {
        self.ui.focus.focused.and_then(|id| self.ui.tree.node(id).and_then(|n| n.key.clone()))
    }

    fn track_inspector_focus(&mut self) {
        let key = self.focused_widget_key();
        let was_editing = self.last_focus_key.as_deref().is_some_and(is_transform_field_key);
        let now_editing = key.as_deref().is_some_and(is_transform_field_key);
        if was_editing && !now_editing {
            self.sync_inspector_transform();
        }
        self.last_focus_key = key;
    }

    fn handle_viewport_input(&mut self, input: &Input) {
        if self.state.center != CenterTab::Scene || self.splitter_drag.is_some() {
            return;
        }

        let rect = center_viewport_rect(self.screen_w, self.screen_h, self.state.dock);
        let (mx, my) = input.mouse_pos();
        if mx < rect.x || my < rect.y || mx > rect.x + rect.w || my > rect.y + rect.h {
            self.viewport_pan = None;
            self.entity_drag = None;
            return;
        }

        let in_scene_body = my > rect.y + 48.0;

        let wheel = input.wheel();
        if wheel != 0.0 {
            let factor = if wheel > 0.0 { 1.1 } else { 0.9 };
            self.state.viewport.zoom = (self.state.viewport.zoom * factor).clamp(0.25, 4.0);
        }

        let panning = input.mouse_down(MouseBtn::Middle)
            || (self.state.tool == Tool::Hand && input.mouse_down(MouseBtn::Left) && in_scene_body);
        if panning {
            if let Some((last_x, last_y)) = self.viewport_pan {
                self.state.viewport.pan_x += mx - last_x;
                self.state.viewport.pan_y += my - last_y;
            }
            self.viewport_pan = Some((mx, my));
            self.entity_drag = None;
        }
        else {
            self.viewport_pan = None;
        }

        let has_selection = entity_by_id(self.project.kind, self.state.selected).is_some();
        let moving_entity = self.state.tool == Tool::Move && has_selection && input.mouse_down(MouseBtn::Left) && in_scene_body && !panning;
        if moving_entity {
            if let Some((last_mx, last_my)) = self.entity_drag {
                let (wx0, wy0) = screen_to_world(last_mx, last_my, rect, &self.state.viewport);
                let (wx1, wy1) = screen_to_world(mx, my, rect, &self.state.viewport);
                self.state.transform.pos_x += wx1 - wx0;
                self.state.transform.pos_y += wy1 - wy0;
                self.push_transform_to_inspector();
            }
            self.entity_drag = Some((mx, my));
        }
        else if self.entity_drag.is_some() {
            self.entity_drag = None;
            self.state.status =
                format!("位置 ({:.1}, {:.1})", self.state.transform.pos_x, self.state.transform.pos_y);
        }

        if input.mouse_pressed(MouseBtn::Left) && in_scene_body && !panning {
            if self.state.tool == Tool::Hand || self.state.tool == Tool::Move {
                let (wx, wy) = screen_to_world(mx, my, rect, &self.state.viewport);
                let pick_radius = 48.0 / self.state.viewport.zoom;
                if let Some(eid) = pick_entity_at_world(self.project.kind, wx, wy, pick_radius) {
                    self.apply_selection(eid);
                }
            }
        }
    }

    fn handle_shortcuts(&mut self, input: &Input) {
        let ctrl = input.key_down(Key::LCtrl) || input.key_down(Key::RCtrl);
        if ctrl && input.key_pressed(Key::J) {
            self.state.dock.bottom_collapsed = !self.state.dock.bottom_collapsed;
            self.state.status = if self.state.dock.bottom_collapsed { "底栏已隐藏 (Ctrl+J)".into() } else { "底栏已显示 (Ctrl+J)".into() };
            self.dirty_ui = true;
            self.persist_dock();
        }
    }

    fn handle_splitter_input(&mut self, input: &Input, screen_w: f32, screen_h: f32) {
        let (mx, my) = input.mouse_pos();

        if let Some(drag) = self.splitter_drag {
            if input.mouse_down(MouseBtn::Left) {
                apply_splitter_drag(drag, mx, my, screen_w, screen_h, &mut self.state.dock);
                self.dirty_ui = true;
            }
            else {
                self.splitter_drag = None;
                self.persist_dock();
            }
            return;
        }

        if input.mouse_pressed(MouseBtn::Left) {
            if let Some(axis) = hit_splitter(input, screen_w, screen_h, self.state.dock) {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
                if let Some((last_axis, last_t)) = self.splitter_last_click {
                    if last_axis == axis && now - last_t < 0.35 {
                        double_click_splitter(axis, &mut self.state.dock);
                        self.dirty_ui = true;
                        self.persist_dock();
                        self.splitter_last_click = None;
                        return;
                    }
                }
                self.splitter_last_click = Some((axis, now));
                self.splitter_drag = Some(SplitterDrag {
                    axis,
                    anchor: match axis {
                        SplitterAxis::Hierarchy | SplitterAxis::Inspector => mx,
                        SplitterAxis::Bottom => my,
                    },
                    start_hierarchy: self.state.dock.hierarchy_width,
                    start_inspector: self.state.dock.inspector_width,
                    start_bottom: self.state.dock.bottom_height,
                });
            }
        }
    }

    fn handle_commands(&mut self) {
        let cmds: Vec<_> = self.ui.drain_commands().collect();
        for cmd in cmds {
            match cmd {
                UiCommand::Custom(CMD_PLAY) => {
                    if self.play.is_none() {
                        self.start_play();
                    }
                    else if self.state.play == PlayMode::Paused {
                        self.state.play = PlayMode::Play;
                        self.state.status = "已继续运行".into();
                        self.dirty_ui = true;
                    }
                    else if self.state.play == PlayMode::Play {
                        self.state.center = CenterTab::Game;
                        self.dirty_ui = true;
                    }
                }
                UiCommand::Custom(CMD_PAUSE) => {
                    if self.play.is_some() && self.state.play == PlayMode::Play {
                        self.state.play = PlayMode::Paused;
                        self.state.status = "已暂停".into();
                        self.dirty_ui = true;
                    }
                }
                UiCommand::Custom(CMD_STEP) => {
                    if self.play.is_some() && self.state.play == PlayMode::Paused {
                        self.state.status = "单步：占位（尚未推进一帧）".into();
                        self.dirty_ui = true;
                    }
                }
                UiCommand::Custom(CMD_STOP) => {
                    if self.play.is_some() {
                        self.stop_play();
                    }
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
                    self.state.status = "窗口：控件图鉴（调试工具）".into();
                    self.state.bottom = BottomTab::Console;
                    self.dirty_ui = true;
                }
                UiCommand::Custom(CMD_LAYOUT_DEFAULT) => {
                    self.apply_layout_preset(LayoutPreset::Default);
                }
                UiCommand::Custom(CMD_LAYOUT_SCRIPT) => {
                    self.apply_layout_preset(LayoutPreset::Script);
                }
                UiCommand::Custom(CMD_LAYOUT_DEBUG) => {
                    self.apply_layout_preset(LayoutPreset::Debug);
                }
                UiCommand::Custom(id) => {
                    if let Some(eid) = parse_select_cmd(id) {
                        self.apply_selection(eid);
                    }
                    else if let Some(idx) = parse_asset_cmd(id) {
                        self.state.selected_asset = Some(idx as u32);
                        if let Some(line) = self.assets.get(idx as usize) {
                            self.state.status = format!("已选资源：{line}");
                        }
                        self.state.bottom = BottomTab::Project;
                        self.dirty_ui = true;
                    }
                    else {
                        self.state.status = format!("未识别命令 {id}");
                        self.dirty_ui = true;
                    }
                }
                other => {
                    tracing::debug!(?other, "未处理 UI 命令");
                }
            }
        }
    }

    fn tick_ui(&mut self, frame: &FrameCtx<'_>) {
        self.screen_w = frame.screen_w;
        self.screen_h = frame.screen_h;
        self.handle_shortcuts(frame.input);
        self.handle_viewport_input(frame.input);
        self.handle_splitter_input(frame.input, frame.screen_w, frame.screen_h);

        if !self.mounted || self.dirty_ui {
            self.remount();
        }

        let ui_frame = UiFrame {
            dt: frame.dt,
            screen_size: Vec2::new(frame.screen_w, frame.screen_h),
            dpi_scale: frame.dpi_scale,
            ui_scale: 1.0,
            safe_area: Insets::default(),
            input: frame.input,
        };

        self.ui.begin_frame(&ui_frame);
        self.ui.dispatch_input(&ui_frame);
        self.track_inspector_focus();
        if frame.input.key_pressed(Key::Enter) {
            self.sync_inspector_transform();
        }
        self.ui.update(frame.dt);
        self.ui.layout(&ui_frame);
        self.handle_commands();
        if self.dirty_ui {
            self.remount();
            self.ui.layout(&ui_frame);
        }
        self.ui.end_frame();
    }

    /// Game 页签且正在 Play / Pause：整窗绘制对局。
    fn show_game_view(&self) -> bool {
        self.play.is_some() && self.state.center == CenterTab::Game
    }
}

impl WindowPump2d for StudioApp {
    fn simulate(&mut self, frame: &FrameCtx<'_>) {
        let immersive = self.play.is_some() && !self.mounted;

        if immersive {
            if self.state.play == PlayMode::Paused {
                return;
            }
            if let Some(play) = self.play.as_mut() {
                play.simulate(frame);
            }
            let stop = self.play.as_ref().map(|p| p.should_exit()).unwrap_or(false);
            if stop || frame.input.key_pressed(Key::Escape) {
                self.exit = true;
            }
            return;
        }

        if frame.input.key_pressed(Key::Escape) {
            if self.play.is_some() {
                self.stop_play();
                self.tick_ui(frame);
                return;
            }
            self.exit = true;
            return;
        }

        self.tick_ui(frame);

        if self.state.play == PlayMode::Paused {
            return;
        }
        if let Some(play) = self.play.as_mut() {
            play.simulate(frame);
            if play.should_exit() {
                self.stop_play();
            }
        }
    }

    fn present_world(&mut self, draw: &mut DrawList) {
        let immersive = self.play.is_some() && !self.mounted;
        if immersive || self.show_game_view() {
            if let Some(play) = self.play.as_mut() {
                play.present_world(draw);
                return;
            }
        }

        if self.state.center == CenterTab::Scene {
            let rect = center_viewport_rect(self.screen_w, self.screen_h, self.state.dock);
            let selection = entity_by_id(self.project.kind, self.state.selected).map(|_| &self.state.transform);
            paint_scene_viewport(draw, rect, &self.state.viewport, self.state.tool, selection);
        }

        self.ui.paint(draw);
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
