//! 游戏装配：资源、仿真系统、2D 绘制系统。
//!
//! 与 `spark-plugin` 的脚本插件不是同一件事。这里的 [`SparkPlugin`] 只向 [`SparkApp`] 登记 Rust 系统。

use spark_ecs::{Schedule, World};

use crate::{
    ecs_host::{AppExit, EcsHost2d},
    render2d::{RenderFrame2d, RenderSchedule2d, RenderSystem2d},
};
use spark_renderer::DrawList;

/// 2D 游戏装配根。建成后交给 [`EcsHost2d`]，不再由游戏持有主循环。
pub struct SparkApp {
    world: World,
    sim: Schedule,
    render: RenderSchedule2d,
}

impl Default for SparkApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkApp {
    pub fn new() -> Self {
        let mut world = World::new();
        world.resources.insert(AppExit::default());
        Self { world, sim: Schedule::new(), render: RenderSchedule2d::new() }
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.world.resources.insert(value);
        self
    }

    pub fn add_system(&mut self, name: &'static str, f: impl FnMut(&mut World) + Send + 'static) -> &mut Self {
        self.sim.add_fn(name, f);
        self
    }

    pub fn add_render_system(&mut self, system: impl RenderSystem2d + 'static) -> &mut Self {
        self.render.add_system(system);
        self
    }

    pub fn add_render_fn(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World, &RenderFrame2d, &mut DrawList) + Send + 'static,
    ) -> &mut Self {
        self.render.add_fn(name, f);
        self
    }

    pub fn add_plugin(&mut self, plugin: &dyn SparkPlugin) -> &mut Self {
        plugin.build(self);
        self
    }

    /// 收成 2D ECS 宿主。固定步长仍由 `run_app_2d` / `LoopedHost2d` 包在外面。
    pub fn into_host(self) -> EcsHost2d {
        EcsHost2d::new(self.world, self.sim).with_renderer(self.render)
    }
}

/// 向 [`SparkApp`] 登记资源与系统。
pub trait SparkPlugin {
    fn build(&self, app: &mut SparkApp);
}
