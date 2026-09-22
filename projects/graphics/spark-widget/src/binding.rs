//! ViewModel 绑定：游戏状态 → Widget 树（不持 `&mut World`）。

use spark_types::Color;

use crate::{
    asset::UiImage,
    command::UiCommandQueue,
    id::WidgetId,
    layout::{Layout, LayoutSpec, Size},
    tree::WidgetTree,
};

/// 每帧在 layout 前同步声明式内容。
pub trait UiViewModel: Send {
    /// 根据外部状态更新树（文本、可见性、`UiImage` 等）。
    fn sync(&mut self, tree: &mut WidgetTree);

    /// 可选：消费本帧命令并写回外部状态。默认空实现。
    fn apply_commands(&mut self, _commands: &mut UiCommandQueue) {}
}

/// 空 ViewModel。
#[derive(Debug, Default, Clone, Copy)]
pub struct NullViewModel;

impl UiViewModel for NullViewModel {
    fn sync(&mut self, _tree: &mut WidgetTree) {}
}

/// 写入节点文案；节点不存在时返回 `false`。
pub fn set_text(tree: &mut WidgetTree, id: WidgetId, text: impl Into<String>) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.content.text = Some(text.into());
    true
}

/// 写入可见性。
pub fn set_visible(tree: &mut WidgetTree, id: WidgetId, visible: bool) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.state.visible = visible;
    true
}

/// 写入禁用态（不接收交互）。
pub fn set_disabled(tree: &mut WidgetTree, id: WidgetId, disabled: bool) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.state.disabled = disabled;
    true
}

/// 写入勾选态（同步 `content` 与 `state`）。
pub fn set_checked(tree: &mut WidgetTree, id: WidgetId, checked: bool) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.content.checked = checked;
    node.state.checked = checked;
    true
}

/// 写入选中伪态。
pub fn set_selected(tree: &mut WidgetTree, id: WidgetId, selected: bool) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.state.selected = selected;
    true
}

/// 写入标量值（Slider / Progress）。
pub fn set_value(tree: &mut WidgetTree, id: WidgetId, value: f32) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.content.value = value;
    true
}

/// 写入图片引用。
pub fn set_image(tree: &mut WidgetTree, id: WidgetId, image: Option<UiImage>) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.content.image = image;
    true
}

/// 写入背景色覆盖。
pub fn set_background(tree: &mut WidgetTree, id: WidgetId, color: Option<Color>) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.style.background = color;
    true
}

/// 写入前景 / 文字色覆盖。
pub fn set_foreground(tree: &mut WidgetTree, id: WidgetId, color: Option<Color>) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.style.foreground = color;
    true
}

/// 写入不透明度覆盖（`0..=1`）。
pub fn set_opacity(tree: &mut WidgetTree, id: WidgetId, opacity: Option<f32>) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.style.opacity = opacity;
    true
}

/// 写入圆角半径覆盖。
pub fn set_corner_radius(tree: &mut WidgetTree, id: WidgetId, radius: Option<f32>) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.style.corner_radius = radius;
    true
}

/// 写入字号覆盖。
pub fn set_font_size(tree: &mut WidgetTree, id: WidgetId, size: Option<f32>) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.style.font_size = size;
    true
}

/// 绝对定位到 `(x, y)`，不改宽高。
pub fn set_absolute_offset(tree: &mut WidgetTree, id: WidgetId, x: f32, y: f32) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.layout.kind = Layout::Absolute;
    node.layout.offset_x = x;
    node.layout.offset_y = y;
    true
}

/// 绝对定位矩形（位置 + 像素宽高）。
pub fn set_absolute_bounds(
    tree: &mut WidgetTree,
    id: WidgetId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.layout = LayoutSpec::absolute_bounds(x, y, width, height);
    true
}

/// 用 [`spark_types::Rect`] 绝对定位（与 [`set_absolute_bounds`] 等价）。
pub fn set_absolute_rect(tree: &mut WidgetTree, id: WidgetId, rect: spark_types::Rect) -> bool {
    set_absolute_bounds(tree, id, rect.x, rect.y, rect.w, rect.h)
}

/// 用完整 [`LayoutSpec`] 覆盖节点布局。
pub fn set_layout(tree: &mut WidgetTree, id: WidgetId, layout: LayoutSpec) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.layout = layout;
    true
}

/// 仅写入像素宽高（保留其余布局字段）。
pub fn set_size_px(tree: &mut WidgetTree, id: WidgetId, width: f32, height: f32) -> bool {
    let Some(node) = tree.node_mut(id) else {
        return false;
    };
    node.layout.width = Size::Px(width);
    node.layout.height = Size::Px(height);
    true
}

/// 自 `root` 子树按 key 查找并写文案。
pub fn set_text_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    text: impl Into<String>,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_text(tree, id, text))
}

/// 自 `root` 子树按 key 查找并写可见性。
pub fn set_visible_by_key(tree: &mut WidgetTree, root: WidgetId, key: &str, visible: bool) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_visible(tree, id, visible))
}

/// 自 `root` 子树按 key 查找并写禁用态。
pub fn set_disabled_by_key(tree: &mut WidgetTree, root: WidgetId, key: &str, disabled: bool) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_disabled(tree, id, disabled))
}

/// 自 `root` 子树按 key 查找并写勾选态。
pub fn set_checked_by_key(tree: &mut WidgetTree, root: WidgetId, key: &str, checked: bool) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_checked(tree, id, checked))
}

/// 自 `root` 子树按 key 查找并写选中态。
pub fn set_selected_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    selected: bool,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_selected(tree, id, selected))
}

/// 自 `root` 子树按 key 查找并写标量值。
pub fn set_value_by_key(tree: &mut WidgetTree, root: WidgetId, key: &str, value: f32) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_value(tree, id, value))
}

/// 自 `root` 子树按 key 查找并写图片。
pub fn set_image_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    image: Option<UiImage>,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_image(tree, id, image))
}

/// 自 `root` 子树按 key 查找并写背景色。
pub fn set_background_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    color: Option<Color>,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_background(tree, id, color))
}

/// 自 `root` 子树按 key 查找并写前景色。
pub fn set_foreground_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    color: Option<Color>,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_foreground(tree, id, color))
}

/// 自 `root` 子树按 key 查找并写不透明度。
pub fn set_opacity_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    opacity: Option<f32>,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_opacity(tree, id, opacity))
}

/// 自 `root` 子树按 key 查找并写圆角。
pub fn set_corner_radius_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    radius: Option<f32>,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_corner_radius(tree, id, radius))
}

/// 自 `root` 子树按 key 查找并写字号。
pub fn set_font_size_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    size: Option<f32>,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_font_size(tree, id, size))
}

/// 自 `root` 子树按 key 查找并绝对定位。
pub fn set_absolute_offset_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    x: f32,
    y: f32,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_absolute_offset(tree, id, x, y))
}

/// 自 `root` 子树按 key 查找并写绝对矩形。
pub fn set_absolute_bounds_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_absolute_bounds(tree, id, x, y, width, height))
}

/// 自 `root` 子树按 key 查找并用 [`spark_types::Rect`] 绝对定位。
pub fn set_absolute_rect_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    rect: spark_types::Rect,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_absolute_rect(tree, id, rect))
}

/// 自 `root` 子树按 key 查找并写像素宽高。
pub fn set_size_px_by_key(
    tree: &mut WidgetTree,
    root: WidgetId,
    key: &str,
    width: f32,
    height: f32,
) -> bool {
    tree.find_by_key(root, key)
        .is_some_and(|id| set_size_px(tree, id, width, height))
}
