//! 检查器演示用 Transform 字段（非 ECS 权威）。

/// 可编辑的 Transform 快照。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformState {
    /// X 位置。
    pub pos_x: f32,
    /// Y 位置。
    pub pos_y: f32,
    /// Z 位置。
    pub pos_z: f32,
    /// X 旋转（度）。
    pub rot_x: f32,
    /// Y 旋转（度）。
    pub rot_y: f32,
    /// Z 旋转（度）。
    pub rot_z: f32,
    /// X 缩放。
    pub scale_x: f32,
    /// Y 缩放。
    pub scale_y: f32,
    /// Z 缩放。
    pub scale_z: f32,
}

impl Default for TransformState {
    fn default() -> Self {
        Self { pos_x: 0.0, pos_y: 0.0, pos_z: 0.0, rot_x: 0.0, rot_y: 0.0, rot_z: 0.0, scale_x: 1.0, scale_y: 1.0, scale_z: 1.0 }
    }
}
