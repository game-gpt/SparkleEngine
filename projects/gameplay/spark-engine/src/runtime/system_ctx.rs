//! Rust 系统受控上下文：不暴露整个引擎，只提供声明过的帧与世界访问。

use spark_ecs::World;
use spark_input::Input;

use crate::frame_state::FrameSnapshot;

use super::commands::RustCommands;

/// Rust 原生系统的单步执行上下文。
///
/// 游戏逻辑应优先通过本类型访问世界，而不是普遍持有 `&mut World`。
/// 早期闭包适配器仍允许 `FnMut(&mut World)`，但新代码应迁移到本 API。
pub struct SystemContext<'a> {
    /// 本仿真步时长（秒）。
    pub dt: f32,
    /// 本步输入快照（从 [`FrameSnapshot`] 克隆，与 `world` 可并存）。
    pub input: Input,
    /// 帧缓冲宽（物理像素）。
    pub screen_w: f32,
    /// 帧缓冲高（物理像素）。
    pub screen_h: f32,
    /// 窗口 DPI 缩放。
    pub dpi_scale: f32,
    /// 权威 ECS 世界（内核拥有；系统按调度相位写入）。
    pub world: &'a mut World,
    /// 本相位共享的结构变更队列（相位末由调度器 [`RustCommands::apply`]）。
    pub commands: &'a mut RustCommands,
}

impl<'a> SystemContext<'a> {
    /// 从已写入世界的 [`FrameSnapshot`] 与外部命令队列构造上下文。
    pub fn from_snapshot(world: &'a mut World, commands: &'a mut RustCommands) -> Option<Self> {
        let snap = world.resources.get::<FrameSnapshot>()?.clone();
        Some(Self {
            dt: snap.dt,
            input: snap.input,
            screen_w: snap.screen_w,
            screen_h: snap.screen_h,
            dpi_scale: snap.dpi_scale,
            world,
            commands,
        })
    }
}
