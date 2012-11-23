//! UI → 游戏命令队列。

use crate::id::WidgetId;

/// 引擎级占位命令。游戏层应定义自己的命令枚举并适配进队列。
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// 关闭当前 overlay / modal。
    CloseOverlay,
    /// 自定义载荷（游戏侧解释）。
    Custom(u64),
    /// 拖放完成：从 `source` 放到 `target`。
    Drop {
        source: WidgetId,
        target: WidgetId,
    },
}

#[derive(Debug, Default)]
pub struct UiCommandQueue {
    commands: Vec<UiCommand>,
}

impl UiCommandQueue {
    pub fn push(&mut self, command: UiCommand) {
        self.commands.push(command);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = UiCommand> + '_ {
        self.commands.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
