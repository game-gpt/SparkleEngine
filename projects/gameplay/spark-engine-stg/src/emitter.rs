//! 弹幕发射模式（几何发射，无具体弹种）。
//!
//! `aim` 与 `origin` 重合时瞄准角视为 0。池满时单发失败被忽略（`spawn` 返回值丢弃）。

use spark_types::Vec2;

use crate::bullet::BulletPool;

/// 几何发射图案。
#[derive(Debug, Clone)]
pub enum EmitPattern {
    /// 单发朝瞄准方向。
    Aimed {
        /// 弹速（世界单位 / 秒）。
        speed: f32,
    },
    /// 扇形：`count` 发，总张角 `spread_rad`，中心朝瞄准。
    Fan {
        /// 发数（内部至少按 1 处理）。
        count: u16,
        /// 总张角（弧度）。
        spread_rad: f32,
        /// 弹速。
        speed: f32,
    },
    /// 环形均分。
    Ring {
        /// 发数。
        count: u16,
        /// 弹速。
        speed: f32,
    },
    /// 螺旋：相对瞄准角再加 `spin`。
    Spiral {
        /// 发数。
        count: u16,
        /// 弹速。
        speed: f32,
        /// 相邻发之间的额外转角（弧度）。
        spin: f32,
    },
}

/// 绑定图案与弹半径 / 层的发射器。
#[derive(Debug, Clone)]
pub struct Emitter {
    /// 本帧发射几何。
    pub pattern: EmitPattern,
    /// 写入池的判定半径。
    pub bullet_radius: f32,
    /// 写入池的分层标签。
    pub layer: u8,
}

impl Emitter {
    /// 向 `pool` 按图案生成若干弹；原点与瞄准点决定基准角。
    pub fn fire(&self, pool: &mut BulletPool, origin: Vec2, aim: Vec2) {
        let base = aim_angle(origin, aim);
        match self.pattern {
            EmitPattern::Aimed { speed } => {
                let (vx, vy) = dir_speed(base, speed);
                let _ = pool.spawn(origin, Vec2::new(vx, vy), self.bullet_radius, self.layer);
            }
            EmitPattern::Fan { count, spread_rad, speed } => {
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
            EmitPattern::Spiral { count, speed, spin } => {
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
    if dx.abs() < 1e-8 && dy.abs() < 1e-8 { 0.0 } else { dy.atan2(dx) }
}

fn dir_speed(angle: f32, speed: f32) -> (f32, f32) {
    (angle.cos() * speed, angle.sin() * speed)
}
