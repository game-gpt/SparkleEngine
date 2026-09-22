//! 物理世界步进。
//!
//! 单线程、固定 `dt` 友好：先积分，再重建宽相，再窄相收集接触。
//! **不做**冲量 / 位置校正；接触列表供上层求解或玩法查询。

use spark_geometry::{aabb_aabb, circle_aabb, circle_circle};
use spark_types::Vec2;

use crate::{
    body::{BodyId, BodyKind, Collider2, RigidBody2},
    broadphase::{Broadphase, UniformGrid},
};

/// 世界级仿真参数。
///
/// 默认重力 `(0, 980)`（世界单位 / 秒²，偏 2D 像素坐标）；`cell_size` 默认 `64`。
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    /// 重力加速度，世界单位 / 秒²；仅作用于 [`BodyKind::Dynamic`]。
    pub gravity: Vec2,
    /// 均匀网格宽相的格子边长，世界单位；构造世界时传入 [`UniformGrid::new`]。
    pub cell_size: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self { gravity: Vec2::new(0.0, 980.0), cell_size: 64.0 }
    }
}

/// 窄相确认的重叠对（无穿透深度 / 法线；后续迭代可扩展）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contact {
    /// 接触对一端。
    pub a: BodyId,
    /// 接触对另一端。
    pub b: BodyId,
}

/// 2D 物理世界：刚体槽、宽相网格、上一帧接触缓存。
///
/// 不变式：`bodies` 与 `free` 共同维护稠密可复用槽；`contacts` 仅在 [`Self::step`] 后有效。
#[derive(Debug)]
pub struct PhysicsWorld {
    /// 可变仿真参数（改 `cell_size` 不会重建已有网格尺寸，需自行换世界或扩展 API）。
    pub config: PhysicsConfig,
    bodies: Vec<Option<RigidBody2>>,
    free: Vec<u32>,
    broadphase: UniformGrid,
    contacts: Vec<Contact>,
}

impl PhysicsWorld {
    /// 用配置创建空世界；宽相 cell 取自 `config.cell_size`。
    pub fn new(config: PhysicsConfig) -> Self {
        let cell = config.cell_size;
        Self { config, bodies: Vec::new(), free: Vec::new(), broadphase: UniformGrid::new(cell), contacts: Vec::new() }
    }

    /// 插入刚体并返回句柄；优先复用已 despawn 的槽。
    pub fn spawn(&mut self, body: RigidBody2) -> BodyId {
        if let Some(slot) = self.free.pop() {
            self.bodies[slot as usize] = Some(body);
            BodyId(slot)
        }
        else {
            let id = self.bodies.len() as u32;
            self.bodies.push(Some(body));
            BodyId(id)
        }
    }

    /// 移除刚体；成功返回 `true`，无效 / 已空槽返回 `false`。
    pub fn despawn(&mut self, id: BodyId) -> bool {
        let Some(slot) = self.bodies.get_mut(id.0 as usize)
        else {
            return false;
        };
        if slot.take().is_some() {
            self.free.push(id.0);
            true
        }
        else {
            false
        }
    }

    /// 只读取刚体；无效句柄或空槽为 `None`。
    pub fn get(&self, id: BodyId) -> Option<&RigidBody2> {
        self.bodies.get(id.0 as usize).and_then(|b| b.as_ref())
    }

    /// 可变取刚体；改位置后建议 [`RigidBody2::sync_collider`]，或等下一次 `step`。
    pub fn get_mut(&mut self, id: BodyId) -> Option<&mut RigidBody2> {
        self.bodies.get_mut(id.0 as usize).and_then(|b| b.as_mut())
    }

    /// 上一帧 [`Self::step`] 产出的接触对切片。
    pub fn contacts(&self) -> &[Contact] {
        &self.contacts
    }

    /// 存活刚体数量（不含空槽）。
    pub fn body_count(&self) -> usize {
        self.bodies.iter().filter(|b| b.is_some()).count()
    }

    /// 积分速度/位置，重建宽相并收集窄相接触（不做冲量求解，留给后续迭代）。
    ///
    /// `dt` 单位为秒。动态体先加重力再积分位置；运动体只按速度平移；静态体只同步碰撞体。
    pub fn step(&mut self, dt: f32) {
        let g = self.config.gravity;
        for body in self.bodies.iter_mut().flatten() {
            if body.kind == BodyKind::Dynamic {
                body.velocity.x += g.x * dt;
                body.velocity.y += g.y * dt;
                body.position.x += body.velocity.x * dt;
                body.position.y += body.velocity.y * dt;
                body.sync_collider();
            }
            else if body.kind == BodyKind::Kinematic {
                body.position.x += body.velocity.x * dt;
                body.position.y += body.velocity.y * dt;
                body.sync_collider();
            }
            else {
                body.sync_collider();
            }
        }

        self.broadphase.clear();
        let mut ids = Vec::new();
        for (i, slot) in self.bodies.iter().enumerate() {
            if let Some(body) = slot {
                let id = BodyId(i as u32);
                self.broadphase.insert(id, body.aabb_bounds());
                ids.push(id);
            }
        }

        self.contacts.clear();
        for (a, b) in self.broadphase.query_pairs() {
            if self.narrowphase(a, b) {
                self.contacts.push(Contact { a, b });
            }
        }
    }

    fn narrowphase(&self, a: BodyId, b: BodyId) -> bool {
        let (Some(ba), Some(bb)) = (self.get(a), self.get(b))
        else {
            return false;
        };
        match (&ba.collider, &bb.collider) {
            (Collider2::Aabb(ra), Collider2::Aabb(rb)) => aabb_aabb(*ra, *rb),
            (Collider2::Circle(ca), Collider2::Circle(cb)) => circle_circle(*ca, *cb),
            (Collider2::Circle(c), Collider2::Aabb(r)) | (Collider2::Aabb(r), Collider2::Circle(c)) => circle_aabb(*c, *r),
        }
    }
}
