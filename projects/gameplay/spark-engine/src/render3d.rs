//! 3D 绘制调度。与 [`crate::RenderSchedule2d`] 对称。
//!
//! Spark 负责顺序。游戏系统决定画什么。

use spark_types::Color;
use spark_ecs::World;
use spark_renderer::DrawList3d;

/// 本帧 3D 绘制上下文。
#[derive(Debug, Clone, Copy)]
pub struct RenderFrame3d {
    pub screen_w: f32,
    pub screen_h: f32,
    pub clear: Color,
}

/// 一个 3D 绘制系统。
pub trait RenderSystem3d: Send {
    fn name(&self) -> &str {
        "render3d"
    }

    fn draw(&mut self, world: &mut World, frame: &RenderFrame3d, draw: &mut DrawList3d);
}

struct FnRender3d<F> {
    name: &'static str,
    f: F,
}

impl<F> RenderSystem3d for FnRender3d<F>
where
    F: FnMut(&mut World, &RenderFrame3d, &mut DrawList3d) + Send,
{
    fn name(&self) -> &str {
        self.name
    }

    fn draw(&mut self, world: &mut World, frame: &RenderFrame3d, draw: &mut DrawList3d) {
        (self.f)(world, frame, draw);
    }
}

/// 有序 3D 绘制系统表。
#[derive(Default)]
pub struct RenderSchedule3d {
    systems: Vec<Box<dyn RenderSystem3d>>,
}

impl RenderSchedule3d {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, system: impl RenderSystem3d + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn add_fn(&mut self, name: &'static str, f: impl FnMut(&mut World, &RenderFrame3d, &mut DrawList3d) + Send + 'static) -> &mut Self {
        self.add_system(FnRender3d { name, f })
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn draw(&mut self, world: &mut World, frame: &RenderFrame3d, draw: &mut DrawList3d) {
        for system in &mut self.systems {
            system.draw(world, frame, draw);
        }
    }
}
