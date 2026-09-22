//! 透视相机（无游戏语义）。

use spark_geometry::{Mat4, Vec3};

/// 第一人称式透视相机：眼点 + yaw/pitch + 透视参数。
#[derive(Debug, Clone)]
pub struct Camera3d {
    /// 世界空间眼点。
    pub eye: Vec3,
    /// 水平朝向（弧度，绕世界 Y）。
    pub yaw: f32,
    /// 俯仰（弧度，已在 [`Self::apply_look`] 中钳制）。
    pub pitch: f32,
    /// 垂直视场角（弧度）。
    pub fov_y_rad: f32,
    /// 近裁剪面距离。
    pub near: f32,
    /// 远裁剪面距离。
    pub far: f32,
}

impl Default for Camera3d {
    fn default() -> Self {
        Self { eye: Vec3::new(0.0, 1.6, 0.0), yaw: 0.0, pitch: 0.0, fov_y_rad: 70f32.to_radians(), near: 0.2, far: 4_000.0 }
    }
}

impl Camera3d {
    /// 单位前向向量（由 yaw/pitch 推出）。
    pub fn forward(&self) -> Vec3 {
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        Vec3::new(sy * cp, sp, -cy * cp).normalized()
    }

    /// 单位右向向量（相对世界上轴）。
    pub fn right(&self) -> Vec3 {
        let mut r = self.forward().cross(Vec3::Y);
        if r.length() < 1e-6 {
            r = self.forward().cross(Vec3::X);
        }
        r.normalized()
    }

    /// 用指针增量更新朝向；超大 delta 丢弃以防捕获瞬间甩飞。
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

    /// 重置朝向为默认略俯视。
    pub fn reset_look(&mut self) {
        self.yaw = 0.0;
        // 略俯视地平线，而不是平视或贴地。
        self.pitch = -0.18;
    }

    /// 视图矩阵（`look_to(eye, forward, up)`）。
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to(self.eye, self.forward(), Vec3::Y)
    }

    /// 透视投影矩阵；`aspect` 过小会被抬到 `0.01`。
    pub fn proj_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective(self.fov_y_rad, aspect.max(0.01), self.near, self.far)
    }

    /// `proj * view`，供 [`crate::DrawList3d::view_proj`] 与裁剪使用。
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.proj_matrix(aspect).mul(self.view_matrix())
    }

    /// 天空用 VP：与主相机同朝向，眼点固定在原点（去掉平移）。
    ///
    /// 天空几何应放在单位球/远球上并以单位 model 提交，避免每帧平移整球。
    pub fn sky_view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_to(Vec3::ZERO, self.forward(), Vec3::Y);
        self.proj_matrix(aspect).mul(view)
    }

    /// 当前相机的视锥（给定宽高比）。
    pub fn frustum(&self, aspect: f32) -> crate::frustum::Frustum {
        crate::frustum::Frustum::from_view_proj(&self.view_proj(aspect))
    }
}
