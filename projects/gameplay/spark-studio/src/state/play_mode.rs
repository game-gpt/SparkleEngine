//! 工具栏 Play 状态机。

/// 工具栏 Play 状态机。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    /// 编辑：不跑游戏主循环。
    Edit,
    /// 播放：进程内嵌入示例 `SparkRuntime`（`RuntimeHost2d`）。
    Play,
    /// 暂停：保留会话，不再 `update`。
    Paused,
}
