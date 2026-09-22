//! ViewModel 绑定：游戏状态 → Widget 树（不持 `&mut World`）。

use crate::{
    asset::UiImage,
    command::UiCommandQueue,
    id::WidgetId,
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
