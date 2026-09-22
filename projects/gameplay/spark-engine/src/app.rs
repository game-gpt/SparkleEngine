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
    /// 空世界 + 空仿真/绘制调度；预插入 [`AppExit`] 资源。
    pub fn new() -> Self {
        let mut world = World::new();
        world.resources.insert(AppExit::default());
        Self { world, sim: Schedule::new(), render: RenderSchedule2d::new() }
    }

    /// 可变访问 ECS 世界（装配期插入资源 / 实体）。
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// 插入资源并返回 `self`，便于链式装配。
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.world.resources.insert(value);
        self
    }

    /// 向仿真 [`Schedule`] 追加命名闭包系统。
    pub fn add_system(&mut self, name: &'static str, f: impl FnMut(&mut World) + Send + 'static) -> &mut Self {
        self.sim.add_fn(name, f);
        self
    }

    /// 向 2D 绘制调度追加实现 [`RenderSystem2d`] 的系统。
    pub fn add_render_system(&mut self, system: impl RenderSystem2d + 'static) -> &mut Self {
        self.render.add_system(system);
        self
    }

    /// 以命名闭包追加 2D 绘制系统。
    pub fn add_render_fn(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World, &RenderFrame2d, &mut DrawList) + Send + 'static,
    ) -> &mut Self {
        self.render.add_fn(name, f);
        self
    }

    /// 调用插件的 [`SparkPlugin::build`] 完成批量登记。
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
    /// 在装配根上插入资源、仿真系统与绘制系统。
    fn build(&self, app: &mut SparkApp);
}
