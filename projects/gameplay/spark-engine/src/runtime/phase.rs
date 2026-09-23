//! 运行时调度相位：Rust 域与 Spark Script 域各自的生命周期槽位。

/// Rust 原生游戏系统所属相位（高性能、权威模拟、渲染准备）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RustPhase {
    /// 帧初：读输入快照、场景切换前置。
    PreUpdate,
    /// 固定步仿真（物理、生产网络、战斗批处理）。
    FixedUpdate,
    /// 可变帧：输入解释、相机、动画表现。
    Update,
    /// 仿真收尾。
    LateUpdate,
    /// 从组件/资源生成绘制缓冲（不直接提交 GPU）。
    RenderPrepare,
    /// Widget 树与 HUD 批次准备。
    UiPrepare,
}

impl RustPhase {
    /// 单帧内 Rust 域执行顺序（不含 Spark Script 穿插）。
    pub const SIM_ORDER: &'static [RustPhase] =
        &[Self::PreUpdate, Self::FixedUpdate, Self::Update, Self::LateUpdate];

    /// 每视觉帧的渲染相位顺序。
    pub const RENDER_ORDER: &'static [RustPhase] = &[Self::RenderPrepare, Self::UiPrepare];
}
