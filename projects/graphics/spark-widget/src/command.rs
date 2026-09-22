//! UI → 游戏命令队列。

use crate::id::WidgetId;

/// 引擎级占位命令。游戏层应定义自己的命令枚举并适配进队列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    /// 关闭当前 overlay / modal。
    CloseOverlay,
    /// 自定义数值载荷（兼容旧代码；新代码优先 [`Self::Action`]）。
    Custom(u64),
    /// 稳定字符串动作名（如 `"title.single"`），便于类型安全映射。
    Action(&'static str),
    /// 拖放完成：从 `source` 放到 `target`。
    Drop {
        /// 拖放源控件。
        source: WidgetId,
        /// 放置目标控件。
        target: WidgetId,
    },
}

impl UiCommand {
    /// 构造 [`Self::Action`]。
    pub fn action(name: &'static str) -> Self {
        Self::Action(name)
    }

    /// 若为 [`Self::Action`] 则返回动作名。
    pub fn as_action(&self) -> Option<&'static str> {
        match self {
            Self::Action(name) => Some(*name),
            _ => None,
        }
    }

    /// 若为 [`Self::Custom`] 则返回数值。
    pub fn as_custom(&self) -> Option<u64> {
        match self {
            Self::Custom(v) => Some(*v),
            _ => None,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_helpers() {
        let cmd = UiCommand::action("title.single");
        assert_eq!(cmd.as_action(), Some("title.single"));
        assert_eq!(cmd.as_custom(), None);
        assert_eq!(UiCommand::Custom(7).as_custom(), Some(7));
    }
}
