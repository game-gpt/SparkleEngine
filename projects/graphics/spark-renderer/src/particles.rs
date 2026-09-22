//! 2D 粒子池：位置、速度、寿命、颜色。
//!
//! 不含伤害数字或具体贴图。绘制时走当前 `DrawList` 图层，世界层会吃相机。

use spark_core::{Color, Rect, Vec2};

use crate::draw::DrawList;

/// 一颗轴对齐粒子。`drag` 为每秒速度衰减比例，`0` 表示不减速。
#[derive(Debug, Clone, Copy)]
pub struct Particle2d {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
    pub size: f32,
    pub drag: f32,
}

impl Particle2d {
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
    pub fn with_capacity(cap: usize) -> Self {
        Self { cap: cap.max(1), particles: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.particles.iter().filter(|p| p.life > 0.0).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 底层槽位（含已过期项），供测试与调试读位置。
    pub fn slots(&self) -> &[Particle2d] {
        &self.particles
    }

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
