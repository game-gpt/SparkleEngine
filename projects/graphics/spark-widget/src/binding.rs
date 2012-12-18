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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{id::WidgetId, widgets::label_widget};

    struct LabelVm {
        text: String,
        target: Option<WidgetId>,
    }

    impl UiViewModel for LabelVm {
        fn sync(&mut self, tree: &mut WidgetTree) {
            let Some(id) = self.target
            else {
                return;
            };
            if let Some(node) = tree.node_mut(id) {
                node.content.text = Some(self.text.clone());
            }
        }
    }

    #[test]
    fn view_model_updates_label_text() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        let id = label_widget().text("old").mount(&mut tree, root).unwrap();
        let mut vm = LabelVm { text: "new".into(), target: Some(id) };
        vm.sync(&mut tree);
        assert_eq!(tree.node(id).unwrap().content.text.as_deref(), Some("new"));
    }
}
