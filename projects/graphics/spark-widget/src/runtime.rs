//! UI 运行时：一帧完整流程入口。

use spark_input::Input;
use spark_renderer::DrawList;
use spark_types::Vec2;

use crate::{
    accessibility::AccessibilityTree,
    asset::{NullTextureResolver, UiTextureResolver},
    binding::{NullViewModel, UiViewModel},
    command::UiCommandQueue,
    drag_drop::DragState,
    event::router,
    focus::FocusManager,
    id::WidgetId,
    inspector::UiInspector,
    motion::MotionManager,
    overlay::OverlayManager,
    paint,
    state::UiState,
    style::Theme,
    text::{EstimateMeasurer, FontMeasurer, MemoryClipboard, TextMeasurer},
    tree::WidgetTree,
};

/// GUI / HUD / Overlay 层标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLayer {
    /// 主界面场景（菜单 / 面板树）。
    Gui,
    /// 游戏内 HUD（血条、准星等，通常不挡世界输入策略由宿主定）。
    Hud,
    /// 浮层（modal / popup / toast / tooltip）。
    Overlay,
}

/// 每帧输入给 runtime 的外部上下文。
pub struct UiFrame<'a> {
    /// 帧间隔（秒），供动效与 toast TTL。
    pub dt: f32,
    /// 逻辑屏幕尺寸（与 layout 同一单位）。
    pub screen_size: Vec2,
    /// 窗口 DPI 缩放（物理像素 / 逻辑像素）。
    pub dpi_scale: f32,
    /// 用户 UI 缩放（叠在 DPI 之上）。
    pub ui_scale: f32,
    /// 安全区 insets（刘海 / 圆角等）。
    pub safe_area: crate::layout::Insets,
    /// 本帧输入快照（只读借用）。
    pub input: &'a Input,
}

/// 每个窗口或渲染表面一个 UI runtime。
pub struct UiRuntime {
    /// 控件树权威。
    pub tree: WidgetTree,
    /// 悬停 / 按下 / 脏标记等帧状态。
    pub state: UiState,
    /// 当前主题。
    pub theme: Theme,
    /// 焦点环与 trap。
    pub focus: FocusManager,
    /// Overlay 栈（modal / popup / toast…）。
    pub overlays: OverlayManager,
    /// UI 动效（非 `spark-animator` 骨骼）。
    pub motion: MotionManager,
    /// 拖放会话状态。
    pub drag: DragState,
    /// 无障碍树（每帧 end 时重建）。
    pub accessibility: AccessibilityTree,
    /// 检视器（Studio / 调试）。
    pub inspector: UiInspector,
    /// 待宿主消费的 UI 命令队列。
    pub commands: UiCommandQueue,
    /// 单调时间（秒），供 toast TTL 等使用。
    pub time: f32,
    /// 文本测量器（默认尝试系统字体，失败则估算）。
    pub text_measurer: Box<dyn TextMeasurer>,
    /// 剪贴板（默认进程内，宿主可替换为系统剪贴板）。
    pub clipboard: Box<dyn crate::text::Clipboard>,
    /// 纹理解析（`AssetId` → GPU `TextureId`）。
    pub textures: Box<dyn UiTextureResolver>,
    /// 可选 ViewModel：layout 前同步树。
    pub view_model: Box<dyn UiViewModel>,
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
            .field("textures", &"<dyn UiTextureResolver>")
            .field("view_model", &"<dyn UiViewModel>")
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
    /// 空运行时：默认主题、空解析器、估算字体（若系统字体可用则用之）。
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
            textures: Box::new(NullTextureResolver),
            view_model: Box::new(NullViewModel),
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

    /// 替换纹理解析器。
    pub fn set_texture_resolver(&mut self, textures: Box<dyn UiTextureResolver>) {
        self.textures = textures;
    }

    /// 替换 ViewModel。
    pub fn set_view_model(&mut self, view_model: Box<dyn UiViewModel>) {
        self.view_model = view_model;
    }

    /// 帧初：清除「输入被 modal 阻断」等瞬态标志（具体事件在 `dispatch_input`）。
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

    /// 按稳定 `key` reconcile GUI scene，尽量保留 `WidgetId` / 焦点 / 悬停。
    ///
    /// 无既有 scene 时退化为 [`Self::mount_scene`]。根 `key`+`kind` 不匹配则整树替换。
    pub fn reconcile_scene(&mut self, builder: crate::widgets::WidgetBuilder) -> Option<WidgetId> {
        let builder = builder.layer(UiLayer::Gui);
        let Some(old) = self.scene_root
        else {
            return self.mount_scene(builder);
        };

        let can_reuse = self.tree.node(old).is_some_and(|n| n.kind == builder.kind() && n.key.as_deref() == builder.key_str());

        if !can_reuse {
            return self.mount_scene(builder);
        }

        let focused_key = self.focus.focused.and_then(|id| self.tree.node(id).and_then(|n| n.key.clone()));
        let hovered_key = self.state.hovered.and_then(|id| self.tree.node(id).and_then(|n| n.key.clone()));

        let root = self.tree.root();
        let id = builder.reconcile(&mut self.tree, root)?;
        self.set_scene_root(id);

        if let Some(key) = focused_key.as_deref() {
            if let Some(fid) = self.tree.find_by_key(id, key) {
                crate::focus::set_focus(&mut self.tree, &mut self.focus, Some(fid));
            }
        }
        if let Some(key) = hovered_key.as_deref() {
            if let Some(hid) = self.tree.find_by_key(id, key) {
                for nid in self.tree.ids() {
                    if let Some(node) = self.tree.node_mut(nid) {
                        node.state.hovered = false;
                    }
                }
                if let Some(node) = self.tree.node_mut(hid) {
                    node.state.hovered = true;
                }
                self.state.hovered = Some(hid);
            }
        }

        self.invalidate_layout();
        Some(id)
    }

    /// 当前 GUI scene 根；未挂载时为 `None`。
    pub fn scene_root(&self) -> Option<WidgetId> {
        self.scene_root
    }

    /// 当前 HUD 根；未挂载时为 `None`。
    pub fn hud_root(&self) -> Option<WidgetId> {
        self.hud_root
    }

    /// 把本帧输入路由进树（悬停、点击、焦点键等）。
    pub fn dispatch_input(&mut self, frame: &UiFrame<'_>) {
        router::dispatch(self, frame);
    }

    /// 推进动效与 overlay TTL；过期 toast/浮层会 `unmount`。
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

    /// 若 layout 脏：同步 ViewModel、定位 overlay、跑布局求解并标记需重绘。
    pub fn layout(&mut self, frame: &UiFrame<'_>) {
        self.view_model.sync(&mut self.tree);
        if !self.state.dirty.layout {
            return;
        }
        let metrics =
            crate::layout::UiMetrics { dpi_scale: frame.dpi_scale.max(0.01), ui_scale: frame.ui_scale.max(0.01), safe_area: frame.safe_area };
        self.overlays.position_anchored(&mut self.tree, frame.screen_size);
        crate::layout::run_layout(&mut self.tree, frame.screen_size, metrics, self.text_measurer.as_mut());
        // 二次定位：measure 后 desired 更准。
        self.overlays.position_anchored(&mut self.tree, frame.screen_size);
        crate::layout::run_layout(&mut self.tree, frame.screen_size, metrics, self.text_measurer.as_mut());
        self.state.dirty.clear_layout();
        self.state.dirty.mark_paint();
    }

    /// 把整棵树画进 `DrawList`（每帧全量遍历；`dirty.paint` 留给日后增量）。
    pub fn paint(&mut self, draw: &mut DrawList) {
        // DrawList 每帧重建：始终遍历绘制。`dirty.paint` 留给增量优化。
        paint::paint_tree(&self.tree, &self.theme, &self.motion, self.textures.as_mut(), draw);
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

    /// 帧末：把命令交给 ViewModel，并重建无障碍树。
    pub fn end_frame(&mut self) {
        self.view_model.apply_commands(&mut self.commands);
        self.accessibility.rebuild_from(&self.tree);
    }

    /// 排空命令队列，供宿主 / 脚本消费。
    pub fn drain_commands(&mut self) -> impl Iterator<Item = crate::command::UiCommand> + '_ {
        self.commands.drain()
    }

    /// 在根下挂载 overlay，并登记到 [`OverlayManager`]。
    pub fn open_overlay(&mut self, layer: crate::overlay::OverlayLayer, builder: crate::widgets::WidgetBuilder) -> Option<WidgetId> {
        let dismiss_on_outside = matches!(layer, crate::overlay::OverlayLayer::Popup | crate::overlay::OverlayLayer::Tooltip);
        self.open_overlay_anchored(layer, None, dismiss_on_outside, builder)
    }

    /// 挂载可贴靠 `anchor` 的 overlay；`dismiss_on_outside` 控制点外关闭。
    ///
    /// Modal 层会把焦点 trap 到该子树。
    pub fn open_overlay_anchored(
        &mut self,
        layer: crate::overlay::OverlayLayer,
        anchor: Option<WidgetId>,
        dismiss_on_outside: bool,
        builder: crate::widgets::WidgetBuilder,
    ) -> Option<WidgetId> {
        let root = self.tree.root();
        let id = builder.layer(UiLayer::Overlay).mount(&mut self.tree, root)?;
        self.overlays.push_entry(crate::overlay::OverlayEntry { id, layer, anchor, dismiss_on_outside, ttl: None });
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
    pub fn show_toast(&mut self, builder: crate::widgets::WidgetBuilder, duration_secs: f32) -> Option<WidgetId> {
        let root = self.tree.root();
        let id = builder.layer(UiLayer::Overlay).mount(&mut self.tree, root)?;
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
    pub fn show_tooltip(&mut self, anchor: WidgetId, builder: crate::widgets::WidgetBuilder) -> Option<WidgetId> {
        for id in self.overlays.remove_layer(crate::overlay::OverlayLayer::Tooltip) {
            self.tree.unmount(id);
        }
        self.open_overlay_anchored(crate::overlay::OverlayLayer::Tooltip, Some(anchor), false, builder)
    }

    /// 打开贴靠 `anchor` 的 popup（点击外部关闭）。
    pub fn show_popup(&mut self, anchor: WidgetId, builder: crate::widgets::WidgetBuilder) -> Option<WidgetId> {
        for id in self.overlays.remove_layer(crate::overlay::OverlayLayer::Popup) {
            self.tree.unmount(id);
        }
        self.open_overlay_anchored(crate::overlay::OverlayLayer::Popup, Some(anchor), true, builder)
    }

    /// 关闭全部 Tooltip 层并卸载对应节点。
    pub fn dismiss_tooltips(&mut self) {
        let removed = self.overlays.remove_layer(crate::overlay::OverlayLayer::Tooltip);
        if removed.is_empty() {
            return;
        }
        for id in removed {
            self.tree.unmount(id);
        }
        self.invalidate_layout();
    }

    /// 关闭指定 overlay 节点并移出栈。
    pub fn close_overlay(&mut self, id: WidgetId) {
        self.overlays.remove(id);
        self.tree.unmount(id);
        self.invalidate_layout();
    }

    /// 弹出并关闭最顶层 overlay；栈空返回 `false`。
    pub fn close_top_overlay(&mut self) -> bool {
        if let Some(entry) = self.overlays.pop_top() {
            self.tree.unmount(entry.id);
            self.invalidate_layout();
            true
        }
        else {
            false
        }
    }
}

fn mark_layer(tree: &mut WidgetTree, id: WidgetId, layer: UiLayer) {
    let children = tree.node(id).map(|n| n.children.clone()).unwrap_or_default();
    if let Some(node) = tree.node_mut(id) {
        node.layer = layer;
    }
    for child in children {
        mark_layer(tree, child, layer);
    }
}

fn default_text_measurer() -> Box<dyn TextMeasurer> {
    if let Some(font) = FontMeasurer::try_system() { Box::new(font) } else { Box::new(EstimateMeasurer) }
}
