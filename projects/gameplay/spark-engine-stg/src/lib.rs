//! Spark **STG**（弹幕射击）特异化引擎壳。
//!
//! 在 [`spark_engine::SparkEngine`] 之上提供：弹幕对象池、发射器、圆判定、关卡时钟与擦弹计数。
//! **禁止**具体弹种表、符卡剧本、角色数值——那些属于游戏仓。

mod bullet;
mod collide;
mod emitter;
mod stage;

pub use bullet::{Bullet, BulletId, BulletPool};
pub use collide::{Circle, collide_circles};
pub use emitter::{EmitPattern, Emitter};
pub use stage::StageClock;

use spark_core::Vec2;
use spark_engine::SparkEngine;
use std::path::PathBuf;

/// STG 会话。
pub struct StgEngine {
    pub engine: SparkEngine,
    pub bullets: BulletPool,
    pub clock: StageClock,
    pub graze: u32,
}

impl StgEngine {
    pub fn new(mods_root: impl Into<PathBuf>, bullet_cap: usize) -> Self {
        Self {
            engine: SparkEngine::new(mods_root),
            bullets: BulletPool::with_capacity(bullet_cap),
            clock: StageClock::default(),
            graze: 0,
        }
    }

    /// 推进关卡时间与弹幕；`player` / `player_r` 用于擦弹与命中检测。
    /// 返回本帧是否命中机体（判定圆）。
    pub fn tick(&mut self, dt: f32, player: Vec2, player_r: f32, graze_r: f32) -> bool {
        self.clock.advance(dt);
        self.bullets.integrate(dt);
        let mut hit = false;
        let player_hurt = Circle {
            center: player,
            radius: player_r,
        };
        let graze_c = Circle {
            center: player,
            radius: graze_r.max(player_r),
        };
        for b in self.bullets.iter_alive() {
            let bc = Circle {
                center: b.pos,
                radius: b.radius,
            };
            if collide_circles(player_hurt, bc) {
                hit = true;
            } else if collide_circles(graze_c, bc) {
                self.graze = self.graze.saturating_add(1);
            }
        }
        self.bullets.despawn_out_of_bounds(-64.0, -64.0, 2000.0, 2000.0);
        hit
    }

    pub fn emit(&mut self, emitter: &Emitter, origin: Vec2, aim: Vec2) {
        emitter.fire(&mut self.bullets, origin, aim);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_and_hit() {
        let mut stg = StgEngine::new(".", 256);
        let em = Emitter {
            pattern: EmitPattern::Fan {
                count: 5,
                spread_rad: std::f32::consts::FRAC_PI_2,
                speed: 100.0,
            },
            bullet_radius: 2.0,
            layer: 0,
        };
        stg.emit(&em, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        assert_eq!(stg.bullets.alive_count(), 5);
        // 子弹打在玩家身上
        let hit = stg.tick(0.0, Vec2::new(0.0, 0.0), 4.0, 8.0);
        assert!(hit);
    }

    #[test]
    fn stage_clock() {
        let mut c = StageClock::default();
        c.advance(0.5);
        assert!((c.time - 0.5).abs() < 1e-5);
        assert!(c.reached(0.4));
        assert!(!c.reached(0.6));
    }
}
