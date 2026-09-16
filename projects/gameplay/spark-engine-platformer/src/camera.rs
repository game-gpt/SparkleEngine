//! 2D 相机死区跟随。

use spark_core::Vec2;

#[derive(Debug, Clone)]
pub struct Camera2d {
    pub pos: Vec2,
    pub deadzone: Vec2,
    pub lerp: f32,
}

impl Default for Camera2d {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            deadzone: Vec2::new(2.0, 1.5),
            lerp: 8.0,
        }
    }
}

impl Camera2d {
    /// 目标超出死区时向目标插值。
    pub fn follow(&mut self, target: Vec2, dt: f32) {
        let mut desired = self.pos;
        let dx = target.x - self.pos.x;
        let dy = target.y - self.pos.y;
        if dx.abs() > self.deadzone.x {
            desired.x = target.x - dx.signum() * self.deadzone.x;
        }
        if dy.abs() > self.deadzone.y {
            desired.y = target.y - dy.signum() * self.deadzone.y;
        }
        let t = (self.lerp * dt).clamp(0.0, 1.0);
        self.pos.x += (desired.x - self.pos.x) * t;
        self.pos.y += (desired.y - self.pos.y) * t;
    }
}
