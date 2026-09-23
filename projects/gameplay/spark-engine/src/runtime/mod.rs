//! Spark 运行时：内核拥有的权威世界与双脚本域调度。
//!
//! - **Rust 域**：原生系统、权威模拟、渲染准备。
//! - **Spark Script 域**：Mod、热更内容与受控命令 API。
//!
//! 窗口泵只通过 [`RuntimeHost2d`] / [`RuntimeHost3d`] 调用 [`SparkRuntime::tick_sim`] 与渲染相位；
//! 游戏不实现 [`spark_renderer::WindowPump2d`] / [`spark_renderer::WindowPump3d`]。

mod host;
mod host3d;
mod native;
mod phase;
mod scene;
mod scheduler;
mod script_domain;
mod system_ctx;
mod commands;
mod loop_system;
mod query;

pub use host::RuntimeHost2d;
pub use host3d::RuntimeHost3d;
pub use native::NativeGamePlugin;
pub use phase::RustPhase;
pub use scene::{SceneCommand, SceneManager, SceneRequests};
pub use scheduler::RuntimeScheduler;
pub use script_domain::SparkScriptDomain;
pub use system_ctx::SystemContext;
pub use commands::RustCommands;
pub use loop_system::{LoopSystem, LoopSystemFn};

use spark_ecs::World;
use spark_renderer::{Camera2d, DrawList, DrawList3d, FrameCtx, UiRenderBatch};
use spark_vm::HostHooks;

use crate::{
    frame_state::{AppExit, CursorGrabPref, DrawBuffer2d, DrawBuffer3d, FrameSnapshot, OsCursorVisible, UiBuffer2d},
    render2d::{RenderFrame2d, RenderSchedule2d},
    render3d::{RenderFrame3d, RenderSchedule3d},
    EngineError, FrameLoopConfig,
};

/// 待执行的场景命令队列（单步最多消费一条）。
#[derive(Debug, Default)]
struct SceneCommandQueue {
    pending: Option<SceneCommand>,
}

/// Spark 引擎运行时：拥有 [`World`]、调度器、场景与 Spark Script 域。
pub struct SparkRuntime {
    world: World,
    scheduler: RuntimeScheduler,
    scenes: SceneManager,
    renderer: RenderSchedule2d,
    renderer_3d: RenderSchedule3d,
    script: SparkScriptDomain,
    loop_config: FrameLoopConfig,
    host_exit: bool,
    cursor_grab: bool,
}

impl Default for SparkRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkRuntime {
    /// 空运行时：预插入 [`AppExit`] 与场景命令队列。
    pub fn new() -> Self {
        let mut world = World::new();
        world.resources.insert(AppExit::default());
        world.resources.insert(SceneCommandQueue::default());
        world.resources.insert(SceneRequests::default());
        world.resources.insert(DrawBuffer2d::default());
        world.resources.insert(UiBuffer2d::default());
        world.resources.insert(DrawBuffer3d::default());
        Self {
            world,
            scheduler: RuntimeScheduler::new(),
            scenes: SceneManager::new(),
            renderer: RenderSchedule2d::new(),
            renderer_3d: RenderSchedule3d::new(),
            script: SparkScriptDomain::new(),
            loop_config: FrameLoopConfig::default(),
            host_exit: false,
            cursor_grab: false,
        }
    }

    /// 设置帧循环策略（在 `run` 前调用）。
    pub fn with_loop_config(mut self, config: FrameLoopConfig) -> Self {
        self.loop_config = config;
        self
    }

    /// 只读访问场景管理器。
    pub fn scenes(&self) -> &SceneManager {
        &self.scenes
    }

    /// 可变访问场景管理器。
    pub fn scenes_mut(&mut self) -> &mut SceneManager {
        &mut self.scenes
    }

    /// 只读访问 Spark Script 域。
    pub fn script_domain(&self) -> &SparkScriptDomain {
        &self.script
    }

    /// 可变访问 Spark Script 域。
    pub fn script_domain_mut(&mut self) -> &mut SparkScriptDomain {
        &mut self.script
    }

    /// 帧循环配置。
    pub fn loop_config(&self) -> &FrameLoopConfig {
        &self.loop_config
    }

    /// 可变访问权威 ECS 世界（装配期使用；运行期优先走系统）。
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// 只读访问世界。
    pub fn world(&self) -> &World {
        &self.world
    }

    /// 插入资源。
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.world.resources.insert(value);
        self
    }

    /// 登记 Rust 原生插件（组件、系统、场景）。
    pub fn register_native<P: NativeGamePlugin>(&mut self, plugin: &P) -> &mut Self {
        plugin.build(self);
        self
    }

    /// 装载 Spark Script 包根（mods 目录）。
    pub fn load_script_package(&mut self, mods_root: impl AsRef<std::path::Path>) -> Result<&mut Self, EngineError> {
        self.script.load_package_root(mods_root)?;
        Ok(self)
    }

    /// 请求加载并切换到场景（等价于写入 [`SceneCommand::Transition`]）。
    pub fn load_scene(&mut self, id: impl Into<String>) -> &mut Self {
        self.enqueue_scene_command(SceneCommand::Transition { to: id.into() });
        self
    }

    /// 向 Rust 相位追加闭包系统（早期 API）。
    pub fn add_rust_system(
        &mut self,
        phase: RustPhase,
        name: &'static str,
        f: impl FnMut(&mut World) + Send + 'static,
    ) -> &mut Self {
        self.scheduler.add_rust_fn(phase, name, f);
        self
    }

    /// 向 Rust 相位追加基于 [`SystemContext`] 的系统。
    pub fn add_rust_system_ctx(
        &mut self,
        phase: RustPhase,
        name: &'static str,
        f: impl FnMut(&mut SystemContext<'_>) + Send + 'static,
    ) -> &mut Self {
        self.scheduler.add_rust_ctx_fn(phase, name, f);
        self
    }

    /// 向 Rust 相位追加 [`LoopSystem`]（语义别名，推荐新代码使用）。
    pub fn add_loop_system<S: LoopSystem + 'static>(&mut self, phase: RustPhase, mut system: S) -> &mut Self {
        let name = system.name();
        self.add_rust_system_ctx(phase, name, move |ctx| system.run(ctx));
        self
    }

    /// 以闭包形式追加 [`LoopSystem`]。
    pub fn add_loop_system_fn(
        &mut self,
        phase: RustPhase,
        name: &'static str,
        f: impl FnMut(&mut SystemContext<'_>) + Send + 'static,
    ) -> &mut Self {
        self.add_loop_system(phase, LoopSystemFn::new(name, f));
        self
    }

    /// 可变访问 2D 渲染调度表（装配 [`RenderSystem2d`] 实现体）。
    pub fn renderer_mut(&mut self) -> &mut RenderSchedule2d {
        &mut self.renderer
    }

    /// 向 2D 渲染准备相位追加绘制系统。
    pub fn add_render_fn(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World, &RenderFrame2d, &mut DrawList) + Send + 'static,
    ) -> &mut Self {
        self.renderer.add_fn(name, f);
        self
    }

    /// 可变访问 3D 渲染调度表（装配 [`RenderSystem3d`] 实现体）。
    pub fn renderer_3d_mut(&mut self) -> &mut RenderSchedule3d {
        &mut self.renderer_3d
    }

    /// 向 3D 渲染准备相位追加绘制系统。
    pub fn add_render_fn_3d(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World, &RenderFrame3d, &mut DrawList3d) + Send + 'static,
    ) -> &mut Self {
        self.renderer_3d.add_fn(name, f);
        self
    }

    /// 提交场景切换命令（Rust 系统或 Spark Script 经宿主 API 写入）。
    pub fn enqueue_scene_command(&mut self, cmd: SceneCommand) {
        if let Some(q) = self.world.resources.get_mut::<SceneCommandQueue>() {
            q.pending = Some(cmd);
        }
    }

    fn drain_scene_command(&mut self) -> Option<SceneCommand> {
        self.world.resources.get_mut::<SceneCommandQueue>().and_then(|q| q.pending.take())
    }

    fn flush_scene_commands(&mut self) {
        let mut batch = Vec::new();
        if let Some(q) = self.world.resources.get_mut::<SceneRequests>() {
            batch.extend(q.pending.drain(..));
        }
        if let Some(cmd) = self.drain_scene_command() {
            batch.push(cmd);
        }
        for cmd in batch {
            self.scenes.flush(&mut self.world, Some(cmd));
        }
    }

    /// 单仿真步：写帧快照 → 场景 → 双域调度。
    pub fn tick_sim(&mut self, frame: &FrameCtx<'_>, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        insert_frame_snapshot(&mut self.world, frame);
        self.flush_scene_commands();
        self.script.begin_frame();
        self.scheduler.run_sim_step(&mut self.world, &mut self.script, host)?;
        Ok(())
    }

    /// 视觉帧末尾：刷新脚本查询快照（供下一帧只读访问）。
    pub fn tick_frame_end(&mut self, frame: &FrameCtx<'_>, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        insert_frame_snapshot(&mut self.world, frame);
        self.flush_scene_commands();
        if self.script.is_loaded() {
            self.script.engine_mut().unwrap().refresh_script_query(&self.world);
        }
        let _ = host;
        Ok(())
    }

    /// 渲染世界层：跑渲染相位后取出 [`DrawBuffer2d`] 或执行 [`RenderSchedule2d`]。
    pub fn render_world(&mut self, draw: &mut DrawList, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        self.scheduler.run_render_frame(&mut self.world, &mut self.script, host)?;
        if let Some(buf) = self.world.resources.get_mut::<DrawBuffer2d>() {
            if let Some(list) = buf.list.take() {
                *draw = list;
                return Ok(());
            }
        }
        if let Some(cam) = self.world.resources.get::<Camera2d>().copied() {
            draw.set_camera(cam);
        }
        if !self.renderer.is_empty() {
            let (screen_w, screen_h) =
                self.world.resources.get::<FrameSnapshot>().map(|s| (s.screen_w, s.screen_h)).unwrap_or((0.0, 0.0));
            let frame = RenderFrame2d { screen_w, screen_h, clear: draw.clear };
            self.renderer.draw(&mut self.world, &frame, draw);
        }
        Ok(())
    }

    /// 渲染 UI 层：取出 [`UiBuffer2d`]。
    pub fn render_ui(&mut self, ui: &mut UiRenderBatch) {
        if let Some(buf) = self.world.resources.get_mut::<UiBuffer2d>() {
            if let Some(batch) = buf.batch.take() {
                *ui = batch;
            }
        }
    }

    /// 渲染 3D 世界层：取出 [`DrawBuffer3d`] 或执行 [`RenderSchedule3d`]。
    pub fn render_world_3d(&mut self, draw: &mut DrawList3d, _host: &mut dyn HostHooks) -> Result<(), EngineError> {
        if let Some(buf) = self.world.resources.get_mut::<DrawBuffer3d>() {
            if let Some(list) = buf.list.take() {
                *draw = list;
                return Ok(());
            }
        }
        if !self.renderer_3d.is_empty() {
            let (screen_w, screen_h) =
                self.world.resources.get::<FrameSnapshot>().map(|s| (s.screen_w, s.screen_h)).unwrap_or((0.0, 0.0));
            let frame = RenderFrame3d { screen_w, screen_h, clear: draw.clear };
            self.renderer_3d.draw(&mut self.world, &frame, draw);
        }
        Ok(())
    }

    /// 是否应退出窗口泵。
    pub fn should_exit(&self) -> bool {
        self.host_exit || self.world.resources.get::<AppExit>().is_some_and(|e| e.requested)
    }

    /// 请求退出。
    pub fn request_exit(&mut self) {
        if let Some(exit) = self.world.resources.get_mut::<AppExit>() {
            exit.request();
        }
        self.host_exit = true;
    }

    /// 操作系统光标是否可见。
    pub fn cursor_visible(&self) -> bool {
        self.world.resources.get::<OsCursorVisible>().map(|c| c.0).unwrap_or(true)
    }

    /// 是否抓取指针。
    pub fn cursor_grab(&self) -> bool {
        if let Some(pref) = self.world.resources.get::<CursorGrabPref>() {
            return pref.0;
        }
        self.cursor_grab
    }

    /// 设置指针抓取偏好（第一人称等）。
    pub fn set_cursor_grab(&mut self, grab: bool) -> &mut Self {
        self.cursor_grab = grab;
        self
    }

    /// 收成 [`RuntimeHost2d`]，供 `spark-renderer-wgpu` 窗口泵驱动。
    pub fn into_host(self) -> RuntimeHost2d {
        let loop_config = self.loop_config.clone();
        RuntimeHost2d::new(self, loop_config)
    }

    /// 收成 [`RuntimeHost3d`]，供 `spark-renderer-wgpu` 3D 窗口泵驱动。
    pub fn into_host_3d(self) -> RuntimeHost3d {
        let loop_config = self.loop_config.clone();
        RuntimeHost3d::new(self, loop_config)
    }
}

fn insert_frame_snapshot(world: &mut World, frame: &FrameCtx<'_>) {
    let snap = FrameSnapshot {
        dt: frame.dt,
        screen_w: frame.screen_w,
        screen_h: frame.screen_h,
        dpi_scale: frame.dpi_scale,
        input: frame.input.clone(),
    };
    world.resources.insert(snap);
}
