//! UI → 游戏命令队列。

use crate::id::WidgetId;

/// 引擎级占位命令。游戏层应定义自己的命令枚举并适配进队列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    /// 关闭当前 overlay / modal。
    CloseOverlay,
    /// 自定义数值载荷（兼容旧代码；新代码优先 [`Self::Action`]）。
    Custom(u64),
    /// 稳定字符串动作名（如 `"title.single"`），可选数值载荷（列表索引等）。
    Action {
        /// 动作名。
        name: &'static str,
        /// 可选载荷（如角色/世界列表索引）。
        payload: Option<u64>,
    },
    /// 拖放完成：从 `source` 放到 `target`。
    Drop {
        /// 拖放源控件。
        source: WidgetId,
        /// 放置目标控件。
        target: WidgetId,
    },
}

impl UiCommand {
    /// 构造无载荷的 [`Self::Action`]。
    pub fn action(name: &'static str) -> Self {
        Self::Action { name, payload: None }
    }

    /// 构造带数值载荷的 [`Self::Action`]。
    pub fn action_with(name: &'static str, payload: u64) -> Self {
        Self::Action {
            name,
            payload: Some(payload),
        }
    }

    /// 若为 [`Self::Action`] 则返回动作名。
    pub fn as_action(&self) -> Option<&'static str> {
        match self {
            Self::Action { name, .. } => Some(*name),
            _ => None,
        }
    }

    /// 若为 [`Self::Action`] 则返回可选载荷。
    pub fn action_payload(&self) -> Option<u64> {
        match self {
            Self::Action { payload, .. } => *payload,
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
        assert_eq!(cmd.action_payload(), None);
        assert_eq!(cmd.as_custom(), None);

        let with = UiCommand::action_with("char.play", 3);
        assert_eq!(with.as_action(), Some("char.play"));
        assert_eq!(with.action_payload(), Some(3));

        assert_eq!(UiCommand::Custom(7).as_custom(), Some(7));
    }
}
