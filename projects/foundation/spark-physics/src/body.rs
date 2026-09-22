//! 2D 刚体。
//!
//! 位置 / 速度单位与世界坐标一致（像素或任意世界单位）；质量仅作数据字段，
//! 当前步进器不做冲量求解，静态体忽略质量。

use spark_geometry::Circle;
use spark_types::{Rect, Vec2};

/// 刚体句柄；与 [`crate::PhysicsWorld`] 槽位索引对应。
///
/// 不变式：`0` 起的稠密索引；despawn 后 ID 可被复用，持有旧句柄的调用方须自行失效。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId(pub u32);

/// 刚体积分类型（决定 `step` 如何改速度与位置）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    /// 受重力、用速度积分位置。
    Dynamic,
    /// 不受重力，仍按速度平移（平台 / 脚本驱动）。
    Kinematic,
    /// 不积分速度与位置；只参与碰撞查询。
    Static,
}

/// 2D 碰撞形状；坐标与 [`RigidBody2::position`] 同步维护。
#[derive(Debug, Clone)]
pub enum Collider2 {
    /// 轴对齐盒（世界空间 `Rect`：`x/y` 为左上，`w/h` 为边长）。
    Aabb(Rect),
    /// 圆（圆心应与刚体 `position` 一致，见 [`RigidBody2::sync_collider`]）。
    Circle(Circle),
}

/// 2D 刚体状态：运动学字段 + 碰撞体。
///
/// 不变式：调用方在改 `position` 后应 [`Self::sync_collider`]，或依赖世界 `step` 末尾的同步。
#[derive(Debug, Clone)]
pub struct RigidBody2 {
    /// 积分类型，见 [`BodyKind`]。
    pub kind: BodyKind,
    /// 质心 / 参考点，世界坐标。
    pub position: Vec2,
    /// 线速度，世界单位 / 秒。
    pub velocity: Vec2,
    /// 碰撞形状（世界空间）。
    pub collider: Collider2,
    /// 质量；静态体忽略。当前步进器尚未用于冲量。
    pub mass: f32,
}

impl RigidBody2 {
    /// 以中心与半尺寸构造 AABB 刚体；初速度为零、质量为 `1.0`。
    ///
    /// `half_extents` 分量应为非负；负值会导致退化 / 翻转矩形。
    pub fn aabb(kind: BodyKind, center: Vec2, half_extents: Vec2) -> Self {
        let rect = Rect::new(center.x - half_extents.x, center.y - half_extents.y, half_extents.x * 2.0, half_extents.y * 2.0);
        Self { kind, position: center, velocity: Vec2::ZERO, collider: Collider2::Aabb(rect), mass: 1.0 }
    }

    /// 以圆心与半径构造圆刚体；初速度为零、质量为 `1.0`。
    pub fn circle(kind: BodyKind, center: Vec2, radius: f32) -> Self {
        Self { kind, position: center, velocity: Vec2::ZERO, collider: Collider2::Circle(Circle::new(center, radius)), mass: 1.0 }
    }

    /// 将碰撞体锚回当前 [`Self::position`]（保持 AABB 半尺寸 / 圆半径不变）。
    pub fn sync_collider(&mut self) {
        match &mut self.collider {
            Collider2::Aabb(r) => {
                let hw = r.w * 0.5;
                let hh = r.h * 0.5;
                r.x = self.position.x - hw;
                r.y = self.position.y - hh;
            }
            Collider2::Circle(c) => {
                c.center = self.position;
            }
        }
    }

    /// 宽相用的轴对齐包围盒（圆取外接正方形）。
    pub fn aabb_bounds(&self) -> Rect {
        match &self.collider {
            Collider2::Aabb(r) => *r,
            Collider2::Circle(c) => Rect::new(c.center.x - c.radius, c.center.y - c.radius, c.radius * 2.0, c.radius * 2.0),
        }
    }
}
