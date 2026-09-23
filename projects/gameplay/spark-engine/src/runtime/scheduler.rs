//! 双域调度器：Rust 相位表 + Spark Script 相位钩子。

use std::collections::HashMap;

use spark_ecs::World;
use spark_script::HostPhase;
use spark_vm::HostHooks;

use super::commands::RustCommands;
use super::phase::RustPhase;
use super::script_domain::SparkScriptDomain;
use super::system_ctx::SystemContext;
use crate::frame_state::FrameSnapshot;
use crate::EngineError;

trait RustRunner: Send {
    fn run(&mut self, world: &mut World, commands: &mut RustCommands);
}

struct WorldFnRunner<F> {
    f: F,
}

impl<F> RustRunner for WorldFnRunner<F>
where
    F: FnMut(&mut World) + Send,
{
    fn run(&mut self, world: &mut World, _commands: &mut RustCommands) {
        (self.f)(world);
    }
}

struct CtxFnRunner<F> {
    f: F,
}

impl<F> RustRunner for CtxFnRunner<F>
where
    F: FnMut(&mut SystemContext<'_>) + Send,
{
    fn run(&mut self, world: &mut World, commands: &mut RustCommands) {
        let Some(snap) = world.resources.get::<FrameSnapshot>().cloned() else {
            return;
        };
        let mut ctx = SystemContext {
            dt: snap.dt,
            input: snap.input,
            screen_w: snap.screen_w,
            screen_h: snap.screen_h,
            dpi_scale: snap.dpi_scale,
            world,
            commands,
        };
        (self.f)(&mut ctx);
    }
}

/// 按相位分桶的 Rust 系统表与 Spark Script 桥。
pub struct RuntimeScheduler {
    rust: HashMap<RustPhase, Vec<Box<dyn RustRunner>>>,
}

impl Default for RuntimeScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeScheduler {
    /// 空调度表。
    pub fn new() -> Self {
        Self { rust: HashMap::new() }
    }

    /// 向指定 Rust 相位追加闭包系统（早期 API：`&mut World`）。
    pub fn add_rust_fn(&mut self, phase: RustPhase, _name: &'static str, f: impl FnMut(&mut World) + Send + 'static) {
        self.rust.entry(phase).or_default().push(Box::new(WorldFnRunner { f }));
    }

    /// 向指定 Rust 相位追加 [`SystemContext`] 系统。
    pub fn add_rust_ctx_fn(&mut self, phase: RustPhase, _name: &'static str, f: impl FnMut(&mut SystemContext<'_>) + Send + 'static) {
        self.rust.entry(phase).or_default().push(Box::new(CtxFnRunner { f }));
    }

    /// 运行单个 Rust 相位（相位末提交 [`RustCommands`]）。
    pub fn run_rust(&mut self, phase: RustPhase, world: &mut World) {
        let mut commands = RustCommands::new();
        if let Some(runners) = self.rust.get_mut(&phase) {
            for runner in runners.iter_mut() {
                runner.run(world, &mut commands);
            }
        }
        commands.apply(world);
    }

    /// 仿真步内按契约顺序驱动 Rust 域与 Spark Script 域。
    ///
    /// 顺序：PreUpdate → Fixed（双域）→ Update（双域）→ LateUpdate（双域）→ 命令提交 → 事件派发。
    pub fn run_sim_step(
        &mut self,
        world: &mut World,
        script: &mut SparkScriptDomain,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        self.run_rust(RustPhase::PreUpdate, world);

        self.run_rust(RustPhase::FixedUpdate, world);
        script.run_phase(HostPhase::FixedUpdate, world, host)?;

        self.run_rust(RustPhase::Update, world);
        script.run_phase(HostPhase::Update, world, host)?;

        self.run_rust(RustPhase::LateUpdate, world);
        script.run_phase(HostPhase::LateUpdate, world, host)?;

        script.apply_commands(world)?;
        script.dispatch_events(host)?;
        Ok(())
    }

    /// 视觉帧渲染相位：Rust 准备 → Spark Script 准备。
    pub fn run_render_frame(
        &mut self,
        world: &mut World,
        script: &mut SparkScriptDomain,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        self.run_rust(RustPhase::RenderPrepare, world);
        script.run_phase(HostPhase::RenderPrepare, world, host)?;
        script.apply_commands(world)?;

        self.run_rust(RustPhase::UiPrepare, world);
        Ok(())
    }
}
