//! Spark **平台跳跃**特异化引擎壳。
//!
//! 轴对齐刚体、固体/单向台、重力跳跃（土狼时间 / 跳跃缓冲）、相机死区跟随。
//! **禁止**关卡块 ID、角色数值表——那些属于游戏仓。

mod actor;
mod camera;
mod world;

pub use actor::{ActorBody, ControllerInput};
pub use camera::Camera2d;
pub use world::{SolidKind, SolidRect, TileWorld};

use spark_core::Vec2;
use spark_engine::SparkEngine;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct PlatformerConfig {
    pub gravity: f32,
    pub move_speed: f32,
    pub jump_speed: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
}

impl Default for PlatformerConfig {
    fn default() -> Self {
        Self { gravity: 40.0, move_speed: 8.0, jump_speed: 14.0, coyote_time: 0.08, jump_buffer: 0.1 }
    }
}

/// 平台跳跃会话。
pub struct PlatformerEngine {
    pub engine: SparkEngine,
    pub world: TileWorld,
    pub player: ActorBody,
    pub camera: Camera2d,
    pub config: PlatformerConfig,
}

impl PlatformerEngine {
    pub fn new(mods_root: impl Into<PathBuf>) -> Self {
        Self {
            engine: SparkEngine::new(mods_root),
            world: TileWorld::default(),
            player: ActorBody::new(Vec2::new(1.0, 2.0), Vec2::new(0.8, 1.2)),
            camera: Camera2d::default(),
            config: PlatformerConfig::default(),
        }
    }

    pub fn tick(&mut self, dt: f32, input: ControllerInput) {
        self.player.integrate(dt, input, &self.config, &self.world);
        self.camera.follow_center(self.player.center(), 0.0, 0.0, crate::camera::DEADZONE, crate::camera::FOLLOW_LERP, dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Rect;

    #[test]
    fn land_and_jump() {
        let mut eng = PlatformerEngine::new(".");
        eng.world.push(SolidRect { rect: Rect::new(0.0, 0.0, 20.0, 1.0), kind: SolidKind::Solid });
        eng.player.pos = Vec2::new(2.0, 3.0);
        eng.player.vel = Vec2::new(0.0, -1.0);
        for _ in 0..30 {
            eng.tick(1.0 / 60.0, ControllerInput::default());
        }
        assert!(eng.player.on_ground);
        let y0 = eng.player.pos.y;
        eng.tick(1.0 / 60.0, ControllerInput { move_x: 0.0, jump_pressed: true, jump_held: true });
        assert!(eng.player.vel.y > 0.0 || eng.player.pos.y > y0);
    }
}
