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

    /// 设置 GUI scene 根，并将子树标记为 [`UiLayer::Gui`]。
    pub fn set_scene_root(&mut self, id: WidgetId) {
        self.scene_root = Some(id);
        mark_layer(&mut self.tree, id, UiLayer::Gui);
    }

    /// 设置 HUD 根，并将子树标记为 [`UiLayer::Hud`]。
    pub fn set_hud_root(&mut self, id: WidgetId) {
        self.hud_root = Some(id);
        mark_layer(&mut self.tree, id, UiLayer::Hud);
    }

    /// 挂载 HUD 场景（替换旧 HUD 根）。
    pub fn mount_hud(&mut self, builder: crate::widgets::WidgetBuilder) -> Option<WidgetId> {
        if let Some(old) = self.hud_root.take() {
            self.tree.unmount(old);
        }
        let root = self.tree.root();
        let id = builder.layer(UiLayer::Hud).mount(&mut self.tree, root)?;
        self.set_hud_root(id);
        Some(id)
    }

    /// 挂载 GUI scene（替换旧 scene 根）。
    pub fn mount_scene(&mut self, builder: crate::widgets::WidgetBuilder) -> Option<WidgetId> {
        if let Some(old) = self.scene_root.take() {
            self.tree.unmount(old);
        }
        let root = self.tree.root();
        let id = builder.layer(UiLayer::Gui).mount(&mut self.tree, root)?;
        self.set_scene_root(id);
        Some(id)
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
        self.overlays
            .position_anchored(&mut self.tree, frame.screen_size);
        crate::layout::run_layout(&mut self.tree, frame.screen_size, frame.dpi_scale);
        // 二次定位：measure 后 desired 更准。
        self.overlays
            .position_anchored(&mut self.tree, frame.screen_size);
        crate::layout::run_layout(&mut self.tree, frame.screen_size, frame.dpi_scale);
    }

    pub fn paint(&mut self, draw: &mut DrawList) {
        paint::paint_tree(&self.tree, &self.theme, draw);
    }

    pub fn end_frame(&mut self) {
        self.accessibility.rebuild_from(&self.tree);
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
        let dismiss_on_outside = matches!(
            layer,
            crate::overlay::OverlayLayer::Popup | crate::overlay::OverlayLayer::Tooltip
        );
        self.open_overlay_anchored(layer, None, dismiss_on_outside, builder)
    }

    pub fn open_overlay_anchored(
        &mut self,
        layer: crate::overlay::OverlayLayer,
        anchor: Option<WidgetId>,
        dismiss_on_outside: bool,
        builder: crate::widgets::WidgetBuilder,
    ) -> Option<WidgetId> {
        let root = self.tree.root();
        let id = builder.layer(UiLayer::Overlay).mount(&mut self.tree, root)?;
        self.overlays.push_entry(crate::overlay::OverlayEntry {
            id,
            layer,
            anchor,
            dismiss_on_outside,
        });
        Some(id)
    }

    /// 打开贴靠 `anchor` 的 tooltip（替换已有 Tooltip 层）。
    pub fn show_tooltip(
        &mut self,
        anchor: WidgetId,
        builder: crate::widgets::WidgetBuilder,
    ) -> Option<WidgetId> {
        for id in self.overlays.remove_layer(crate::overlay::OverlayLayer::Tooltip) {
            self.tree.unmount(id);
        }
        self.open_overlay_anchored(
            crate::overlay::OverlayLayer::Tooltip,
            Some(anchor),
            false,
            builder,
        )
    }

    /// 打开贴靠 `anchor` 的 popup（点击外部关闭）。
    pub fn show_popup(
        &mut self,
        anchor: WidgetId,
        builder: crate::widgets::WidgetBuilder,
    ) -> Option<WidgetId> {
        for id in self.overlays.remove_layer(crate::overlay::OverlayLayer::Popup) {
            self.tree.unmount(id);
        }
        self.open_overlay_anchored(
            crate::overlay::OverlayLayer::Popup,
            Some(anchor),
            true,
            builder,
        )
    }

    pub fn dismiss_tooltips(&mut self) {
        for id in self.overlays.remove_layer(crate::overlay::OverlayLayer::Tooltip) {
            self.tree.unmount(id);
        }
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

fn mark_layer(tree: &mut WidgetTree, id: WidgetId, layer: UiLayer) {
    let children = tree
        .node(id)
        .map(|n| n.children.clone())
        .unwrap_or_default();
    if let Some(node) = tree.node_mut(id) {
        node.layer = layer;
    }
    for child in children {
        mark_layer(tree, child, layer);
    }
}
