//! 3D 游戏装配。与 [`crate::SparkApp`] 对称，不替代 2D 入口。

use spark_ecs::{Schedule, World};
use spark_renderer::DrawList3d;

use crate::{
    ecs_host::{AppExit, EcsHost3d},
    render3d::{RenderFrame3d, RenderSchedule3d, RenderSystem3d},
};

/// 3D 游戏装配根。建成后交给 [`EcsHost3d`]。
pub struct SparkApp3d {
    world: World,
    sim: Schedule,
    render: RenderSchedule3d,
}

impl Default for SparkApp3d {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkApp3d {
    /// 空世界 + 空仿真/绘制调度；预插入 [`AppExit`] 资源。
    pub fn new() -> Self {
        let mut world = World::new();
        world.resources.insert(AppExit::default());
        Self { world, sim: Schedule::new(), render: RenderSchedule3d::new() }
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

    /// 向 3D 绘制调度追加实现 [`RenderSystem3d`] 的系统。
    pub fn add_render_system(&mut self, system: impl RenderSystem3d + 'static) -> &mut Self {
        self.render.add_system(system);
        self
    }

    /// 以命名闭包追加 3D 绘制系统。
    pub fn add_render_fn(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World, &RenderFrame3d, &mut DrawList3d) + Send + 'static,
    ) -> &mut Self {
        self.render.add_fn(name, f);
        self
    }

    /// 调用插件的 [`SparkPlugin3d::build`] 完成批量登记。
    pub fn add_plugin(&mut self, plugin: &dyn SparkPlugin3d) -> &mut Self {
        plugin.build(self);
        self
    }

    /// 收成 3D ECS 宿主；固定步长仍由 `run_app_3d` / `LoopedHost3d` 外包。
    pub fn into_host(self) -> EcsHost3d {
        EcsHost3d::new(self.world, self.sim).with_renderer(self.render)
    }
}

/// 向 [`SparkApp3d`] 登记资源与系统。
pub trait SparkPlugin3d {
    /// 在装配根上插入资源、仿真系统与绘制系统。
    fn build(&self, app: &mut SparkApp3d);
}
