//! UI 运行时：一帧完整流程入口。

use spark_core::Vec2;
use spark_input::Input;
use spark_renderer::DrawList;

use crate::accessibility::AccessibilityTree;
use crate::command::UiCommandQueue;
use crate::event::router;
use crate::focus::FocusManager;
use crate::id::WidgetId;
use crate::motion::MotionManager;
use crate::overlay::OverlayManager;
use crate::paint;
use crate::state::UiState;
use crate::style::Theme;
use crate::tree::WidgetTree;

/// GUI / HUD / Overlay 层标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLayer {
    Gui,
    Hud,
    Overlay,
}

/// 每帧输入给 runtime 的外部上下文。
pub struct UiFrame<'a> {
    pub dt: f32,
    pub screen_size: Vec2,
    pub dpi_scale: f32,
    pub input: &'a Input,
}

/// 每个窗口或渲染表面一个 UI runtime。
#[derive(Debug)]
pub struct UiRuntime {
    pub tree: WidgetTree,
    pub state: UiState,
    pub theme: Theme,
    pub focus: FocusManager,
    pub overlays: OverlayManager,
    pub motion: MotionManager,
    pub accessibility: AccessibilityTree,
    pub commands: UiCommandQueue,
    scene_root: Option<WidgetId>,
    hud_root: Option<WidgetId>,
}

impl Default for UiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl UiRuntime {
    pub fn new() -> Self {
        Self {
            tree: WidgetTree::new(),
            state: UiState::default(),
            theme: Theme::default(),
            focus: FocusManager::default(),
            overlays: OverlayManager::default(),
            motion: MotionManager::default(),
            accessibility: AccessibilityTree::default(),
            commands: UiCommandQueue::default(),
            scene_root: None,
            hud_root: None,
        }
    }

    pub fn begin_frame(&mut self, _frame: &UiFrame<'_>) {
        self.state.input_blocked = false;
    }

    /// 设置 GUI scene 根（占位：仅记录 ID）。
    pub fn set_scene_root(&mut self, id: WidgetId) {
        self.scene_root = Some(id);
    }

    /// 设置 HUD 根（占位：仅记录 ID）。
    pub fn set_hud_root(&mut self, id: WidgetId) {
        self.hud_root = Some(id);
    }

    pub fn scene_root(&self) -> Option<WidgetId> {
        self.scene_root
    }

    pub fn hud_root(&self) -> Option<WidgetId> {
        self.hud_root
    }

    pub fn dispatch_input(&mut self, frame: &UiFrame<'_>) {
        router::dispatch(self, frame);
    }

    pub fn update(&mut self, dt: f32) {
        self.motion.tick(dt);
    }

    pub fn layout(&mut self, frame: &UiFrame<'_>) {
        crate::layout::run_layout(&mut self.tree, frame.screen_size, frame.dpi_scale);
    }

    pub fn paint(&mut self, draw: &mut DrawList) {
        paint::paint_tree(&self.tree, &self.theme, draw);
    }

    pub fn end_frame(&mut self) {
        // 占位：后续做失效清理、命令提交边界等。
    }

    pub fn drain_commands(&mut self) -> impl Iterator<Item = crate::command::UiCommand> + '_ {
        self.commands.drain()
    }

    /// 在根下挂载 overlay，并登记到 [`OverlayManager`]。
    pub fn open_overlay(
        &mut self,
        layer: crate::overlay::OverlayLayer,
        builder: crate::widgets::WidgetBuilder,
    ) -> Option<WidgetId> {
        let root = self.tree.root();
        let id = builder.mount(&mut self.tree, root)?;
        self.overlays.push(id, layer);
        Some(id)
    }

    pub fn close_overlay(&mut self, id: WidgetId) {
        self.overlays.remove(id);
        self.tree.unmount(id);
    }

    pub fn close_top_overlay(&mut self) -> bool {
        if let Some(entry) = self.overlays.pop_top() {
            self.tree.unmount(entry.id);
            true
        } else {
            false
        }
    }
}
