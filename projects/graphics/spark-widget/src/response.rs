//! 事件响应：是否吞掉、是否继续冒泡、默认行为。

use crate::command::UiCommand;

/// 单次事件处理后的控制标志。
#[derive(Debug, Default)]
pub struct EventResponse {
    /// 本节点（或合并路径）已处理该事件。
    pub handled: bool,
    /// 停止沿祖先冒泡。
    pub stop_propagation: bool,
    /// 阻止 router 默认行为（如按钮发命令、滚动偏移）。
    pub prevent_default: bool,
    /// 本次处理产生的、待入队的 UI 命令。
    pub commands: Vec<UiCommand>,
}

impl EventResponse {
    /// 标记已处理，但继续冒泡且允许默认行为。
    pub fn handled() -> Self {
        Self { handled: true, stop_propagation: false, prevent_default: false, commands: Vec::new() }
    }

    /// 标记已处理并停止冒泡。
    pub fn stop() -> Self {
        Self { handled: true, stop_propagation: true, prevent_default: false, commands: Vec::new() }
    }

    /// 标记已处理并阻止 router 默认行为。
    pub fn prevent() -> Self {
        Self { handled: true, stop_propagation: false, prevent_default: true, commands: Vec::new() }
    }

    /// 标记已处理并附带一条待派发命令。
    pub fn with_command(command: UiCommand) -> Self {
        Self { handled: true, stop_propagation: false, prevent_default: false, commands: vec![command] }
    }

    /// 将另一份响应按位或合并进自身，并追加其命令列表。
    pub fn merge(&mut self, other: EventResponse) {
        self.handled |= other.handled;
        self.stop_propagation |= other.stop_propagation;
        self.prevent_default |= other.prevent_default;
        self.commands.extend(other.commands);
    }
}
