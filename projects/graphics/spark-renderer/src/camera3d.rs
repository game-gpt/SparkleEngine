//! 透视相机（无游戏语义）。

use spark_geometry::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct Camera3d {
    pub eye: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y_rad: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera3d {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 1.6, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            fov_y_rad: 70f32.to_radians(),
            near: 0.2,
            far: 4_000.0,
        }
    }
}

impl Camera3d {
    pub fn forward(&self) -> Vec3 {
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        Vec3::new(sy * cp, sp, -cy * cp).normalized()
    }

    pub fn right(&self) -> Vec3 {
        let mut r = self.forward().cross(Vec3::Y);
        if r.length() < 1e-6 {
            r = self.forward().cross(Vec3::X);
        }
        r.normalized()
    }

    pub fn apply_look(&mut self, dx: f32, dy: f32, sens: f32) {
        // 指针捕获瞬间常有巨量 delta，直接丢弃避免俯仰甩飞到脚底。
        const MAX_D: f32 = 80.0;
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        if dx.abs() > MAX_D || dy.abs() > MAX_D {
            return;
        }
        self.yaw += dx * sens;
        self.pitch = (self.pitch - dy * sens).clamp(-1.2, 1.2);
    }

    pub fn reset_look(&mut self) {
        self.yaw = 0.0;
        // 略俯视地平线，而不是平视或贴地。
        self.pitch = -0.18;
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to(self.eye, self.forward(), Vec3::Y)
    }

    pub fn proj_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective(self.fov_y_rad, aspect.max(0.01), self.near, self.far)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.proj_matrix(aspect).mul(self.view_matrix())
    }
}
