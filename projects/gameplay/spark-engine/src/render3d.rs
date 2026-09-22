//! 3D 绘制调度。与 [`crate::RenderSchedule2d`] 对称。
//!
//! Spark 负责顺序。游戏系统决定画什么。

use spark_ecs::World;
use spark_renderer::DrawList3d;
use spark_types::Color;

/// 本帧 3D 绘制上下文。
#[derive(Debug, Clone, Copy)]
pub struct RenderFrame3d {
    /// 物理像素宽。
    pub screen_w: f32,
    /// 物理像素高。
    pub screen_h: f32,
    /// 本帧清屏色。
    pub clear: Color,
}

/// 一个 3D 绘制系统。
pub trait RenderSystem3d: Send {
    /// 调试 / 日志用短名；默认 `"render3d"`。
    fn name(&self) -> &str {
        "render3d"
    }

    /// 把本系统可见的状态写入 `draw`。
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
    /// 空调度。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个绘制系统（注册顺序即执行顺序）。
    pub fn add_system(&mut self, system: impl RenderSystem3d + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    /// 以命名闭包追加绘制系统。
    pub fn add_fn(&mut self, name: &'static str, f: impl FnMut(&mut World, &RenderFrame3d, &mut DrawList3d) + Send + 'static) -> &mut Self {
        self.add_system(FnRender3d { name, f })
    }

    /// 是否尚未登记任何系统。
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// 已登记系统数。
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// 按注册顺序绘制到同一张 `DrawList3d`。
    pub fn draw(&mut self, world: &mut World, frame: &RenderFrame3d, draw: &mut DrawList3d) {
        for system in &mut self.systems {
            system.draw(world, frame, draw);
        }
    }
}
