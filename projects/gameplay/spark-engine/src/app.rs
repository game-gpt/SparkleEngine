//! 游戏装配：资源、仿真系统、2D 绘制系统。
//!
//! 与 `spark-plugin` 的脚本插件不是同一件事。这里的 [`SparkPlugin`] 只向 [`SparkApp`] 登记 Rust 系统。
//! 内部委托 [`SparkRuntime`]；新游戏优先 [`NativeGamePlugin`] + [`run_runtime`]。

use spark_ecs::World;
use spark_renderer::DrawList;

use crate::{
    ecs_host::EcsHost2d,
    render2d::{RenderFrame2d, RenderSystem2d},
    runtime::{RustPhase, SparkRuntime},
};

/// 2D 游戏装配根（[`SparkRuntime`] 的兼容薄封装）。
pub struct SparkApp {
    runtime: SparkRuntime,
}

impl Default for SparkApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkApp {
    /// 空世界 + 空仿真/绘制调度；预插入 [`AppExit`] 等资源。
    pub fn new() -> Self {
        Self { runtime: SparkRuntime::new() }
    }

    /// 可变访问 ECS 世界（装配期插入资源 / 实体）。
    pub fn world_mut(&mut self) -> &mut World {
        self.runtime.world_mut()
    }

    /// 插入资源并返回 `self`，便于链式装配。
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.runtime.insert_resource(value);
        self
    }

    /// 向仿真 [`Schedule`] 追加命名闭包系统（映射到 [`RustPhase::Update`]）。
    pub fn add_system(&mut self, name: &'static str, f: impl FnMut(&mut World) + Send + 'static) -> &mut Self {
        self.runtime.add_rust_system(RustPhase::Update, name, f);
        self
    }

    /// 向 2D 绘制调度追加实现 [`RenderSystem2d`] 的系统。
    pub fn add_render_system(&mut self, system: impl RenderSystem2d + 'static) -> &mut Self {
        self.runtime.renderer_mut().add_system(system);
        self
    }

    /// 以命名闭包追加 2D 绘制系统。
    pub fn add_render_fn(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World, &RenderFrame2d, &mut DrawList) + Send + 'static,
    ) -> &mut Self {
        self.runtime.add_render_fn(name, f);
        self
    }

    /// 调用插件的 [`SparkPlugin::build`] 完成批量登记。
    pub fn add_plugin(&mut self, plugin: &dyn SparkPlugin) -> &mut Self {
        plugin.build(self);
        self
    }

    /// 收成 [`SparkRuntime`]（推荐：`run_runtime`）。
    pub fn into_runtime(self) -> SparkRuntime {
        self.runtime
    }

    /// 收成 2D ECS 宿主（兼容：`run_app_2d` 旧路径）。
    pub fn into_host(self) -> EcsHost2d {
        self.runtime.into_ecs_host()
    }
}

/// 向 [`SparkApp`] 登记资源与系统。
pub trait SparkPlugin {
    /// 在装配根上插入资源、仿真系统与绘制系统。
    fn build(&self, app: &mut SparkApp);
}
