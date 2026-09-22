//! Spark **平台跳跃**特异化引擎壳。
//!
//! 轴对齐刚体、固体/单向台、重力跳跃（土狼时间 / 跳跃缓冲）、相机死区跟随。
//! **禁止**关卡块 ID、角色数值表——那些属于游戏仓。

#![forbid(missing_docs)]
mod actor;
mod camera;
mod world;

pub use actor::{ActorBody, ControllerInput};
pub use camera::Camera2d;
pub use world::{SolidKind, SolidRect, TileWorld};

use spark_engine::SparkEngine;
use spark_types::Vec2;
use std::path::PathBuf;

/// 平台跳跃物理调参（世界单位 / 秒）。
#[derive(Debug, Clone, Copy)]
pub struct PlatformerConfig {
    /// 向下重力加速度。
    pub gravity: f32,
    /// 水平最大速度（`move_x` 满量程时）。
    pub move_speed: f32,
    /// 起跳初速度（向上为正，与重力符号约定一致）。
    pub jump_speed: f32,
    /// 离地后仍可起跳的宽限秒数（土狼时间）。
    pub coyote_time: f32,
    /// 提前按跳键的缓冲秒数。
    pub jump_buffer: f32,
}

impl Default for PlatformerConfig {
    fn default() -> Self {
        Self { gravity: 40.0, move_speed: 8.0, jump_speed: 14.0, coyote_time: 0.08, jump_buffer: 0.1 }
    }
}

/// 平台跳跃会话。
pub struct PlatformerEngine {
    /// 底层模组 / 帧循环宿主。
    pub engine: SparkEngine,
    /// 固体碰撞世界。
    pub world: TileWorld,
    /// 玩家刚体。
    pub player: ActorBody,
    /// 跟随相机。
    pub camera: Camera2d,
    /// 物理调参。
    pub config: PlatformerConfig,
}

impl PlatformerEngine {
    /// 空世界 + 默认玩家尺寸，相机与调参取默认值。
    pub fn new(mods_root: impl Into<PathBuf>) -> Self {
        Self {
            engine: SparkEngine::new(mods_root),
            world: TileWorld::default(),
            player: ActorBody::new(Vec2::new(1.0, 2.0), Vec2::new(0.8, 1.2)),
            camera: Camera2d::default(),
            config: PlatformerConfig::default(),
        }
    }

    /// 一帧：刚体积分后相机跟玩家中心（使用模块默认死区 / 插值）。
    pub fn tick(&mut self, dt: f32, input: ControllerInput) {
        self.player.integrate(dt, input, &self.config, &self.world);
        self.camera.follow_center(self.player.center(), 0.0, 0.0, crate::camera::DEADZONE, crate::camera::FOLLOW_LERP, dt);
    }
}
