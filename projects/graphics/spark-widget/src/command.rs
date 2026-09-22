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
        /// 拖放源控件。
        source: WidgetId,
        /// 放置目标控件。
        target: WidgetId,
    },
}

/// 本帧由 Widget / router 写入、由游戏层 `drain` 消费的命令队列。
#[derive(Debug, Default)]
pub struct UiCommandQueue {
    commands: Vec<UiCommand>,
}

impl UiCommandQueue {
    /// 追加一条命令到队尾。
    pub fn push(&mut self, command: UiCommand) {
        self.commands.push(command);
    }

    /// 取出并清空全部待消费命令。
    pub fn drain(&mut self) -> impl Iterator<Item = UiCommand> + '_ {
        self.commands.drain(..)
    }

    /// 队列是否为空。
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
