//! 2D 绘制调度：按注册顺序把游戏状态写成 `DrawList`。
//!
//! Spark 负责顺序与 `DrawList` 生命周期。游戏系统只决定画什么。
//! 允许 `&mut World`：图集上传和帧准备仍可能改资源，不在这里另开宿主。

use spark_ecs::World;
use spark_renderer::DrawList;
use spark_types::Color;

/// 本帧绘制上下文（从窗口与帧快照整理，不含输入生命周期）。
#[derive(Debug, Clone, Copy)]
pub struct RenderFrame2d {
    /// 物理像素宽。
    pub screen_w: f32,
    /// 物理像素高。
    pub screen_h: f32,
    /// 本帧清屏色（通常来自 `DrawList::clear`）。
    pub clear: Color,
}

/// 一个 2D 绘制系统。
pub trait RenderSystem2d: Send {
    /// 调试 / 日志用短名；默认 `"render"`。
    fn name(&self) -> &str {
        "render"
    }

    /// 把本系统可见的状态写入 `draw`。
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
    /// 空调度。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个绘制系统（注册顺序即执行顺序）。
    pub fn add_system(&mut self, system: impl RenderSystem2d + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    /// 以命名闭包追加绘制系统。
    pub fn add_fn(&mut self, name: &'static str, f: impl FnMut(&mut World, &RenderFrame2d, &mut DrawList) + Send + 'static) -> &mut Self {
        self.add_system(FnRender2d { name, f })
    }

    /// 是否尚未登记任何系统。
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// 已登记系统数。
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
