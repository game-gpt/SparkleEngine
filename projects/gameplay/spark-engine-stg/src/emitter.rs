//! 弹幕发射模式（几何发射，无具体弹种）。

use spark_core::Vec2;

use crate::bullet::BulletPool;

#[derive(Debug, Clone)]
pub enum EmitPattern {
    /// 单发朝瞄准方向。
    Aimed { speed: f32 },
    /// 扇形：`count` 发，总张角 `spread_rad`，中心朝瞄准。
    Fan {
        count: u16,
        spread_rad: f32,
        speed: f32,
    },
    /// 环形均分。
    Ring { count: u16, speed: f32 },
    /// 螺旋：相对瞄准角再加 `spin`。
    Spiral {
        count: u16,
        speed: f32,
        spin: f32,
    },
}

#[derive(Debug, Clone)]
pub struct Emitter {
    pub pattern: EmitPattern,
    pub bullet_radius: f32,
    pub layer: u8,
}

impl Emitter {
    pub fn fire(&self, pool: &mut BulletPool, origin: Vec2, aim: Vec2) {
        let base = aim_angle(origin, aim);
        match self.pattern {
            EmitPattern::Aimed { speed } => {
                let (vx, vy) = dir_speed(base, speed);
                let _ = pool.spawn(origin, Vec2::new(vx, vy), self.bullet_radius, self.layer);
            }
            EmitPattern::Fan {
                count,
                spread_rad,
                speed,
            } => {
                let n = count.max(1);
                if n == 1 {
                    let (vx, vy) = dir_speed(base, speed);
                    let _ = pool.spawn(origin, Vec2::new(vx, vy), self.bullet_radius, self.layer);
                    return;
                }
                let start = base - spread_rad * 0.5;
                let step = spread_rad / (n - 1) as f32;
                for i in 0..n {
                    let a = start + step * i as f32;
                    let (vx, vy) = dir_speed(a, speed);
                    let _ = pool.spawn(origin, Vec2::new(vx, vy), self.bullet_radius, self.layer);
                }
            }
            EmitPattern::Ring { count, speed } => {
                let n = count.max(1) as f32;
                let step = std::f32::consts::TAU / n;
                for i in 0..count.max(1) {
                    let a = base + step * i as f32;
                    let (vx, vy) = dir_speed(a, speed);
                    let _ = pool.spawn(origin, Vec2::new(vx, vy), self.bullet_radius, self.layer);
                }
            }
            EmitPattern::Spiral {
                count,
                speed,
                spin,
            } => {
                let n = count.max(1);
                for i in 0..n {
                    let a = base + spin * i as f32;
                    let (vx, vy) = dir_speed(a, speed);
                    let _ = pool.spawn(origin, Vec2::new(vx, vy), self.bullet_radius, self.layer);
                }
            }
        }
    }
}

fn aim_angle(origin: Vec2, aim: Vec2) -> f32 {
    let dx = aim.x - origin.x;
    let dy = aim.y - origin.y;
    if dx.abs() < 1e-8 && dy.abs() < 1e-8 {
        0.0
    } else {
        dy.atan2(dx)
    }
}

fn dir_speed(angle: f32, speed: f32) -> (f32, f32) {
    (angle.cos() * speed, angle.sin() * speed)
}
