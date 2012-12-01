//! UI 运行时：一帧完整流程入口。

use spark_core::Vec2;
use spark_input::Input;
use spark_renderer::DrawList;

use crate::accessibility::AccessibilityTree;
use crate::command::UiCommandQueue;
use crate::drag_drop::DragState;
use crate::event::router;
use crate::focus::FocusManager;
use crate::id::WidgetId;
use crate::inspector::UiInspector;
use crate::motion::MotionManager;
use crate::overlay::OverlayManager;
use crate::paint;
use crate::state::UiState;
use crate::style::Theme;
use crate::text::{EstimateMeasurer, FontMeasurer, MemoryClipboard, TextMeasurer};
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
    pub ui_scale: f32,
    pub safe_area: crate::layout::Insets,
    pub input: &'a Input,
}

/// 每个窗口或渲染表面一个 UI runtime。
pub struct UiRuntime {
    pub tree: WidgetTree,
    pub state: UiState,
    pub theme: Theme,
    pub focus: FocusManager,
    pub overlays: OverlayManager,
    pub motion: MotionManager,
    pub drag: DragState,
    pub accessibility: AccessibilityTree,
    pub inspector: UiInspector,
    pub commands: UiCommandQueue,
    /// 单调时间（秒），供 toast TTL 等使用。
    pub time: f32,
    /// 文本测量器（默认尝试系统字体，失败则估算）。
    pub text_measurer: Box<dyn TextMeasurer>,
    /// 剪贴板（默认进程内，宿主可替换为系统剪贴板）。
    pub clipboard: Box<dyn crate::text::Clipboard>,
    scene_root: Option<WidgetId>,
    hud_root: Option<WidgetId>,
}

impl std::fmt::Debug for UiRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiRuntime")
            .field("tree", &self.tree)
            .field("state", &self.state)
            .field("theme", &self.theme)
            .field("focus", &self.focus)
            .field("overlays", &self.overlays)
            .field("motion", &self.motion)
            .field("drag", &self.drag)
            .field("accessibility", &self.accessibility)
            .field("inspector", &self.inspector)
            .field("commands", &self.commands)
            .field("time", &self.time)
            .field("text_measurer", &"<dyn TextMeasurer>")
            .field("clipboard", &"<dyn Clipboard>")
            .field("scene_root", &self.scene_root)
            .field("hud_root", &self.hud_root)
            .finish()
    }
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
            drag: DragState::default(),
            accessibility: AccessibilityTree::default(),
            inspector: UiInspector::new(),
            commands: UiCommandQueue::default(),
            time: 0.0,
            text_measurer: default_text_measurer(),
            clipboard: Box::new(MemoryClipboard::new()),
            scene_root: None,
            hud_root: None,
        }
    }

    /// 替换文本测量器（测试或自定义字体）。
    pub fn set_text_measurer(&mut self, measurer: Box<dyn TextMeasurer>) {
        self.text_measurer = measurer;
    }

    /// 替换剪贴板实现。
    pub fn set_clipboard(&mut self, clipboard: Box<dyn crate::text::Clipboard>) {
        self.clipboard = clipboard;
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
        self.invalidate_layout();
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
        self.invalidate_layout();
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
        self.time += dt;
        self.motion.sync_and_tick(&self.tree, dt);
        let expired = self.overlays.tick(dt);
        if !expired.is_empty() {
            for id in expired {
                self.tree.unmount(id);
            }
            self.invalidate_layout();
        }
    }

    pub fn layout(&mut self, frame: &UiFrame<'_>) {
        if !self.state.dirty.layout {
            return;
        }
        let metrics = crate::layout::UiMetrics {
            dpi_scale: frame.dpi_scale.max(0.01),
            ui_scale: frame.ui_scale.max(0.01),
            safe_area: frame.safe_area,
        };
        self.overlays
            .position_anchored(&mut self.tree, frame.screen_size);
        crate::layout::run_layout(
            &mut self.tree,
            frame.screen_size,
            metrics,
            self.text_measurer.as_mut(),
        );
        // 二次定位：measure 后 desired 更准。
        self.overlays
            .position_anchored(&mut self.tree, frame.screen_size);
        crate::layout::run_layout(
            &mut self.tree,
            frame.screen_size,
            metrics,
            self.text_measurer.as_mut(),
        );
        self.state.dirty.clear_layout();
        self.state.dirty.mark_paint();
    }

    pub fn paint(&mut self, draw: &mut DrawList) {
        // DrawList 每帧重建：始终遍历绘制。`dirty.paint` 留给增量优化。
        paint::paint_tree(&self.tree, &self.theme, &self.motion, draw);
        self.state.dirty.clear_paint();
    }

    /// 标记需要重新 layout（mount / 内容变化后调用）。
    pub fn invalidate_layout(&mut self) {
        self.state.dirty.mark_layout();
    }

    /// 仅标记需要重绘。
    pub fn invalidate_paint(&mut self) {
        self.state.dirty.mark_paint();
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
        let id = builder
            .layer(UiLayer::Overlay)
            .mount(&mut self.tree, root)?;
        self.overlays.push_entry(crate::overlay::OverlayEntry {
            id,
            layer,
            anchor,
            dismiss_on_outside,
            ttl: None,
        });
        if layer == crate::overlay::OverlayLayer::Modal {
            crate::focus::ensure_focus_in_trap(&self.tree, &mut self.focus, id);
            for node_id in self.tree.ids() {
                if let Some(node) = self.tree.node_mut(node_id) {
                    node.state.focused = Some(node_id) == self.focus.focused;
                }
            }
        }
        self.invalidate_layout();
        Some(id)
    }

    /// 打开 toast（默认 2.5s 后自动关闭）。
    pub fn show_toast(
        &mut self,
        builder: crate::widgets::WidgetBuilder,
        duration_secs: f32,
    ) -> Option<WidgetId> {
        let root = self.tree.root();
        let id = builder
            .layer(UiLayer::Overlay)
            .mount(&mut self.tree, root)?;
        self.overlays.push_entry(crate::overlay::OverlayEntry {
            id,
            layer: crate::overlay::OverlayLayer::Toast,
            anchor: None,
            dismiss_on_outside: false,
            ttl: Some(duration_secs.max(0.1)),
        });
        Some(id)
    }

    /// 打开贴靠 `anchor` 的 tooltip（替换已有 Tooltip 层）。
    pub fn show_tooltip(
        &mut self,
        anchor: WidgetId,
        builder: crate::widgets::WidgetBuilder,
    ) -> Option<WidgetId> {
        for id in self
            .overlays
            .remove_layer(crate::overlay::OverlayLayer::Tooltip)
        {
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
        for id in self
            .overlays
            .remove_layer(crate::overlay::OverlayLayer::Popup)
        {
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
        let removed = self
            .overlays
            .remove_layer(crate::overlay::OverlayLayer::Tooltip);
        if removed.is_empty() {
            return;
        }
        for id in removed {
            self.tree.unmount(id);
        }
        self.invalidate_layout();
    }

    pub fn close_overlay(&mut self, id: WidgetId) {
        self.overlays.remove(id);
        self.tree.unmount(id);
        self.invalidate_layout();
    }

    pub fn close_top_overlay(&mut self) -> bool {
        if let Some(entry) = self.overlays.pop_top() {
            self.tree.unmount(entry.id);
            self.invalidate_layout();
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

fn default_text_measurer() -> Box<dyn TextMeasurer> {
    if let Some(font) = FontMeasurer::try_system() {
        Box::new(font)
    } else {
        Box::new(EstimateMeasurer)
    }
}
