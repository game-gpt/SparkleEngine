//! ViewModel 绑定：游戏状态 → Widget 树（不持 `&mut World`）。

use crate::{command::UiCommandQueue, tree::WidgetTree};

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
