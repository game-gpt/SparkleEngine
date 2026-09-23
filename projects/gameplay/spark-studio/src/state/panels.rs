//! 中央与底部面板标签枚举。

/// 中央工作区标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterTab {
    /// 场景视图（编辑器视口）。
    Scene,
    /// 游戏视图（Play 画面）。
    Game,
    /// 脚本 / 检视辅助页。
    Script,
}

/// 底部面板标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    /// 项目资源树。
    Project,
    /// 控制台日志。
    Console,
    /// 诊断 / Problems。
    Problems,
}
