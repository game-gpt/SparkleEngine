//! 2D 视口：世界坐标与屏幕像素的变换，以及朝目标的死区跟随。
//!
//! 不含关卡、角色或操作手感。`origin` 是视口左上角的世界坐标。

use spark_types::Vec2;

/// 正交 2D 相机。`zoom` 为世界单位到像素的缩放，`1` 表示一比一。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2d {
    pub origin: Vec2,
    pub zoom: f32,
}

impl Default for Camera2d {
    fn default() -> Self {
        Self { origin: Vec2::ZERO, zoom: 1.0 }
    }
}

impl Camera2d {
    pub fn new(origin: Vec2, zoom: f32) -> Self {
        Self { origin, zoom: sanitize_zoom(zoom) }
    }

    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        let z = sanitize_zoom(self.zoom);
        Vec2::new((world.x - self.origin.x) * z, (world.y - self.origin.y) * z)
    }

    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        let z = sanitize_zoom(self.zoom);
        Vec2::new(screen.x / z + self.origin.x, screen.y / z + self.origin.y)
    }

    /// 让 `target` 留在视口中心附近。
    ///
    /// `deadzone` 是相对中心的世界单位半宽与半高。`screen_w` / `screen_h` 为像素。
    /// 屏幕尺寸为 0 时，`origin` 本身就是跟随点。
    pub fn follow_center(&mut self, target: Vec2, screen_w: f32, screen_h: f32, deadzone: Vec2, lerp: f32, dt: f32) {
        let z = sanitize_zoom(self.zoom);
        self.zoom = z;
        let half = Vec2::new(screen_w * 0.5 / z, screen_h * 0.5 / z);
        let center = Vec2::new(self.origin.x + half.x, self.origin.y + half.y);
        let mut desired = center;
        let dx = target.x - center.x;
        let dy = target.y - center.y;
        if dx.abs() > deadzone.x {
            desired.x = target.x - dx.signum() * deadzone.x;
        }
        if dy.abs() > deadzone.y {
            desired.y = target.y - dy.signum() * deadzone.y;
        }
        let t = (lerp * dt).clamp(0.0, 1.0);
        let next = Vec2::new(center.x + (desired.x - center.x) * t, center.y + (desired.y - center.y) * t);
        self.origin = Vec2::new(next.x - half.x, next.y - half.y);
    }
}

fn sanitize_zoom(zoom: f32) -> f32 {
    if zoom.is_finite() && zoom > 1.0e-6 { zoom } else { 1.0 }
}
