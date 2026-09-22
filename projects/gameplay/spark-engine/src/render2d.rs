//! 2D 绘制调度：按注册顺序把游戏状态写成 `DrawList`。
//!
//! Spark 负责顺序与 `DrawList` 生命周期。游戏系统只决定画什么。
//! 允许 `&mut World`：图集上传和帧准备仍可能改资源，不在这里另开宿主。

use spark_core::Color;
use spark_ecs::World;
use spark_renderer::DrawList;

/// 本帧绘制上下文（从窗口与帧快照整理，不含输入生命周期）。
#[derive(Debug, Clone, Copy)]
pub struct RenderFrame2d {
    pub screen_w: f32,
    pub screen_h: f32,
    pub clear: Color,
}

/// 一个 2D 绘制系统。
pub trait RenderSystem2d: Send {
    fn name(&self) -> &str {
        "render"
    }

    fn draw(&mut self, world: &mut World, frame: &RenderFrame2d, draw: &mut DrawList);
}

struct FnRender2d<F> {
    name: &'static str,
    f: F,
}

impl<F> RenderSystem2d for FnRender2d<F>
where
    F: FnMut(&mut World, &RenderFrame2d, &mut DrawList) + Send,
{
    fn name(&self) -> &str {
        self.name
    }

    fn draw(&mut self, world: &mut World, frame: &RenderFrame2d, draw: &mut DrawList) {
        (self.f)(world, frame, draw);
    }
}

/// 有序 2D 绘制系统表。
#[derive(Default)]
pub struct RenderSchedule2d {
    systems: Vec<Box<dyn RenderSystem2d>>,
}

impl RenderSchedule2d {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, system: impl RenderSystem2d + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn add_fn(&mut self, name: &'static str, f: impl FnMut(&mut World, &RenderFrame2d, &mut DrawList) + Send + 'static) -> &mut Self {
        self.add_system(FnRender2d { name, f })
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// 按注册顺序绘制到同一张 `DrawList`。
    pub fn draw(&mut self, world: &mut World, frame: &RenderFrame2d, draw: &mut DrawList) {
        for system in &mut self.systems {
            system.draw(world, frame, draw);
        }
    }
}
