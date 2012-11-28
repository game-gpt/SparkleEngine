//! 事件响应：是否吞掉、是否继续冒泡、默认行为。

use crate::command::UiCommand;

/// 单次事件处理后的控制标志。
#[derive(Debug, Default)]
pub struct EventResponse {
    pub handled: bool,
    /// 停止沿祖先冒泡。
    pub stop_propagation: bool,
    /// 阻止 router 默认行为（如按钮发命令、滚动偏移）。
    pub prevent_default: bool,
    pub commands: Vec<UiCommand>,
}

impl EventResponse {
    pub fn handled() -> Self {
        Self {
            handled: true,
            stop_propagation: false,
            prevent_default: false,
            commands: Vec::new(),
        }
    }

    pub fn stop() -> Self {
        Self {
            handled: true,
            stop_propagation: true,
            prevent_default: false,
            commands: Vec::new(),
        }
    }

    pub fn with_command(command: UiCommand) -> Self {
        Self {
            handled: true,
            stop_propagation: false,
            prevent_default: false,
            commands: vec![command],
        }
    }

    pub fn merge(&mut self, other: EventResponse) {
        self.handled |= other.handled;
        self.stop_propagation |= other.stop_propagation;
        self.prevent_default |= other.prevent_default;
        self.commands.extend(other.commands);
    }
}
