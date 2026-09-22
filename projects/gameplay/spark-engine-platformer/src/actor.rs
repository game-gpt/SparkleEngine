//! 玩家 / 角色刚体。
//!
//! 分轴解析（先 X 后 Y）；单向台仅在下落且脚贴台顶附近时站立。短跳：松开跳跃且上升中时竖直速度减半。

use spark_geometry::aabb_aabb;
use spark_types::{Rect, Vec2};

use crate::{
    PlatformerConfig,
    world::{SolidKind, TileWorld},
};

/// 一帧控制输入（由宿主从输入系统映射）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ControllerInput {
    /// 水平意图，建议钳到 `[-1, 1]`（积分内仍会 clamp）。
    pub move_x: f32,
    /// 本帧是否新按下跳跃。
    pub jump_pressed: bool,
    /// 跳跃键是否仍按住（用于可变跳高）。
    pub jump_held: bool,
}

/// 轴对齐角色刚体（位置为 AABB 左下角）。
#[derive(Debug, Clone)]
pub struct ActorBody {
    /// 世界坐标左下角。
    pub pos: Vec2,
    /// AABB 宽高。
    pub size: Vec2,
    /// 速度（世界单位 / 秒）。
    pub vel: Vec2,
    /// 上一帧竖直解析后是否站在地面 / 单向台上。
    pub on_ground: bool,
    coyote: f32,
    jump_buf: f32,
}

impl ActorBody {
    /// 静止刚体；土狼与跳缓初始为 0。
    pub fn new(pos: Vec2, size: Vec2) -> Self {
        Self { pos, size, vel: Vec2::ZERO, on_ground: false, coyote: 0.0, jump_buf: 0.0 }
    }

    /// 当前轴对齐包围盒。
    pub fn aabb(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, self.size.x, self.size.y)
    }

    /// AABB 中心点（相机跟随用）。
    pub fn center(&self) -> Vec2 {
        self.aabb().center()
    }

    /// 按输入与重力积分一帧，并对 [`TileWorld`] 固体做分轴碰撞。
    ///
    /// `dt` 负值按 0 处理。起跳消耗土狼与跳缓计时。
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
