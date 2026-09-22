//! 玩家 / 角色刚体。

use spark_types::{Rect, Vec2};
use spark_geometry::aabb_aabb;

use crate::{
    PlatformerConfig,
    world::{SolidKind, TileWorld},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ControllerInput {
    pub move_x: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
}

#[derive(Debug, Clone)]
pub struct ActorBody {
    pub pos: Vec2,
    pub size: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    coyote: f32,
    jump_buf: f32,
}

impl ActorBody {
    pub fn new(pos: Vec2, size: Vec2) -> Self {
        Self { pos, size, vel: Vec2::ZERO, on_ground: false, coyote: 0.0, jump_buf: 0.0 }
    }

    pub fn aabb(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, self.size.x, self.size.y)
    }

    pub fn center(&self) -> Vec2 {
        self.aabb().center()
    }

    pub fn integrate(&mut self, dt: f32, input: ControllerInput, cfg: &PlatformerConfig, world: &TileWorld) {
        let dt = dt.max(0.0);
        self.vel.x = input.move_x.clamp(-1.0, 1.0) * cfg.move_speed;
        self.vel.y -= cfg.gravity * dt;

        if input.jump_pressed {
            self.jump_buf = cfg.jump_buffer;
        }
        else {
            self.jump_buf = (self.jump_buf - dt).max(0.0);
        }
        if self.on_ground {
            self.coyote = cfg.coyote_time;
        }
        else {
            self.coyote = (self.coyote - dt).max(0.0);
        }
        if self.jump_buf > 0.0 && self.coyote > 0.0 {
            self.vel.y = cfg.jump_speed;
            self.on_ground = false;
            self.coyote = 0.0;
            self.jump_buf = 0.0;
        }
        if !input.jump_held && self.vel.y > 0.0 {
            self.vel.y *= 0.5;
        }

        // 分轴：先 X 后 Y
        self.pos.x += self.vel.x * dt;
        self.resolve_x(world);
        self.pos.y += self.vel.y * dt;
        self.on_ground = false;
        self.resolve_y(world);
    }

    fn resolve_x(&mut self, world: &TileWorld) {
        let mut box_ = self.aabb();
        for s in world.solids() {
            if s.kind == SolidKind::OneWay {
                continue;
            }
            if !aabb_aabb(box_, s.rect) {
                continue;
            }
            if self.vel.x > 0.0 {
                self.pos.x = s.rect.x - self.size.x;
            }
            else if self.vel.x < 0.0 {
                self.pos.x = s.rect.x + s.rect.w;
            }
            self.vel.x = 0.0;
            box_ = self.aabb();
        }
    }

    fn resolve_y(&mut self, world: &TileWorld) {
        let mut box_ = self.aabb();
        for s in world.solids() {
            if !aabb_aabb(box_, s.rect) {
                continue;
            }
            match s.kind {
                SolidKind::Solid => {
                    if self.vel.y < 0.0 {
                        self.pos.y = s.rect.y + s.rect.h;
                        self.on_ground = true;
                    }
                    else if self.vel.y > 0.0 {
                        self.pos.y = s.rect.y - self.size.y;
                    }
                    self.vel.y = 0.0;
                    box_ = self.aabb();
                }
                SolidKind::OneWay => {
                    // 仅下落且脚在台顶附近
                    if self.vel.y <= 0.0 {
                        let feet = self.pos.y;
                        let top = s.rect.y + s.rect.h;
                        if feet >= top - 0.2 && feet <= top + 0.05 {
                            self.pos.y = top;
                            self.vel.y = 0.0;
                            self.on_ground = true;
                            box_ = self.aabb();
                        }
                    }
                }
            }
        }
    }
}
