//! 2D 粒子池：位置、速度、寿命、颜色。
//!
//! 不含伤害数字或具体贴图。绘制时走当前 `DrawList` 图层，世界层会吃相机。

use spark_types::{Color, Rect, Vec2};

use crate::draw::DrawList;

/// 一颗轴对齐粒子。`drag` 为每秒速度衰减比例，`0` 表示不减速。
#[derive(Debug, Clone, Copy)]
pub struct Particle2d {
    /// 中心位置。世界层为世界单位，HUD 为屏幕像素（与当前 [`DrawList`] 层一致）。
    pub pos: Vec2,
    /// 速度（单位 / 秒）；与 `pos` 同属当前绘制层坐标空间。
    pub vel: Vec2,
    /// 剩余寿命（秒）。`≤ 0` 视为死亡，可被池复用。
    pub life: f32,
    /// 初始寿命（秒），用于 `draw` 时按剩余比例衰减 alpha。
    pub max_life: f32,
    /// 基础颜色；绘制时 alpha 再乘 `life / max_life`。
    pub color: Color,
    /// 方块边长（同坐标空间单位）；中心在 `pos`。
    pub size: f32,
    /// 每秒速度衰减比例：`vel *= (1 - drag * dt)`，钳到 `[0,1]`。
    pub drag: f32,
}

impl Particle2d {
    /// 构造粒子：`life` 同时写入 `max_life`，`drag` 默认为 `0`。
    pub fn new(pos: Vec2, vel: Vec2, life: f32, color: Color, size: f32) -> Self {
        Self { pos, vel, life, max_life: life.max(0.0), color, size, drag: 0.0 }
    }
}

/// 固定容量的粒子池。满了就替换剩余寿命最短的一颗。
#[derive(Debug, Clone)]
pub struct ParticlePool2d {
    cap: usize,
    particles: Vec<Particle2d>,
}

impl ParticlePool2d {
    /// 预分配容量至少为 `1` 的空池；超出容量时 `spawn` 覆盖最短寿命槽。
    pub fn with_capacity(cap: usize) -> Self {
        Self { cap: cap.max(1), particles: Vec::new() }
    }

    /// 当前存活粒子数（`life > 0`）。
    pub fn len(&self) -> usize {
        self.particles.iter().filter(|p| p.life > 0.0).count()
    }

    /// 是否没有任何存活粒子。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 底层槽位（含已过期项），供测试与调试读位置。
    pub fn slots(&self) -> &[Particle2d] {
        &self.particles
    }

    /// 生成一颗粒子：优先复用死亡槽，其次扩容至 `cap`，否则替换最短寿命者。
    pub fn spawn(&mut self, particle: Particle2d) {
        if let Some(slot) = self.particles.iter_mut().find(|p| p.life <= 0.0) {
            *slot = particle;
            return;
        }
        if self.particles.len() < self.cap {
            self.particles.push(particle);
            return;
        }
        if let Some(slot) = self.particles.iter_mut().min_by(|a, b| a.life.partial_cmp(&b.life).unwrap_or(std::cmp::Ordering::Equal)) {
            *slot = particle;
        }
    }

    /// 推进模拟：扣寿命、按 `drag` 衰减速度、积分位置。`dt` 为秒，负值按 `0`。
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for p in &mut self.particles {
            if p.life <= 0.0 {
                continue;
            }
            p.life -= dt;
            let keep = (1.0 - p.drag * dt).clamp(0.0, 1.0);
            p.vel.x *= keep;
            p.vel.y *= keep;
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
        }
    }

    /// 按剩余寿命缩放 alpha，画成方块。
    pub fn draw(&self, draw: &mut DrawList) {
        for p in &self.particles {
            if p.life <= 0.0 || p.max_life <= 0.0 {
                continue;
            }
            let t = (p.life / p.max_life).clamp(0.0, 1.0);
            let mut color = p.color;
            color.a *= t;
            let s = p.size.max(0.0);
            draw.fill_rect(Rect::new(p.pos.x - s * 0.5, p.pos.y - s * 0.5, s, s), color);
        }
    }
}
