//! 双域调度器：Rust / Spark Script 统一调度图 + 相位驱动。

use std::collections::HashMap;

use spark_ecs::World;
use spark_vm::HostHooks;

use crate::ScriptSystemDescriptor;

use super::commands::RustCommands;
use super::phase::RustPhase;
use super::system_schedule::{SystemOrder, SystemSchedule};
use super::schedule_graph::{MixedNode, host_phase_for_rust, is_mixed_rust_phase, ordered_mixed_phase};
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

struct RegisteredSystem {
    schedule: SystemSchedule,
    runner: Box<dyn RustRunner>,
}

/// 帧循环调度表：Rust System 与 Spark Script System 统一混排。
pub struct RuntimeScheduler {
    systems: HashMap<RustPhase, Vec<RegisteredSystem>>,
}

impl Default for RuntimeScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeScheduler {
    /// 空调度表。
    pub fn new() -> Self {
        Self { systems: HashMap::new() }
    }

    /// 向指定 Rust 相位追加闭包系统（早期 API：`&mut World`）。
    pub fn add_fn(&mut self, phase: RustPhase, name: &'static str, f: impl FnMut(&mut World) + Send + 'static) {
        self.systems.entry(phase).or_default().push(RegisteredSystem {
            schedule: SystemSchedule::new(name, phase),
            runner: Box::new(WorldFnRunner { f }),
        });
    }

    /// 向指定 Rust 相位追加 [`SystemContext`] 系统。
    pub fn add_ctx_fn(&mut self, phase: RustPhase, name: &'static str, f: impl FnMut(&mut SystemContext<'_>) + Send + 'static) {
        self.systems.entry(phase).or_default().push(RegisteredSystem {
            schedule: SystemSchedule::new(name, phase),
            runner: Box::new(CtxFnRunner { f }),
        });
    }

    /// 向指定 Rust 相位追加带顺序约束的 [`SystemContext`] 系统。
    pub fn add_ctx_fn_with_order(
        &mut self,
        phase: RustPhase,
        name: &'static str,
        order: &SystemOrder,
        f: impl FnMut(&mut SystemContext<'_>) + Send + 'static,
    ) {
        self.systems.entry(phase).or_default().push(RegisteredSystem {
            schedule: SystemSchedule::with_order(name, phase, order),
            runner: Box::new(CtxFnRunner { f }),
        });
    }

    /// 运行仅 Rust 的相位（相位末提交 [`RustCommands`]）。
    pub fn run_rust_only(&mut self, phase: RustPhase, world: &mut World) {
        let mut commands = RustCommands::new();
        if let Some(entries) = self.systems.get_mut(&phase) {
            for entry in entries.iter_mut() {
                entry.runner.run(world, &mut commands);
            }
        }
        commands.apply(world);
    }

    /// 运行 Rust 与 Spark Script 混排相位。
    pub fn run_mixed_phase(
        &mut self,
        phase: RustPhase,
        world: &mut World,
        script: &mut SparkScriptDomain,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        let host_phase = host_phase_for_rust(phase).expect("caller ensures mixed phase");
        let native_entries = self.systems.get(&phase).map(|v| v.as_slice()).unwrap_or(&[]);
        let native_schedules: Vec<SystemSchedule> = native_entries.iter().map(|e| e.schedule.clone()).collect();

        let script_descs: Vec<ScriptSystemDescriptor> = if let Some(engine) = script.engine() {
            engine.script_systems().for_phase(host_phase).cloned().collect()
        }
        else {
            Vec::new()
        };

        let order = ordered_mixed_phase(&native_schedules, &script_descs).map_err(EngineError::ScriptSystem)?;

        if script.is_loaded() {
            script.engine_mut().unwrap().refresh_script_query(world);
        }

        let mut commands = RustCommands::new();
        for node in order {
            match node {
                MixedNode::Native(i) => {
                    if let Some(entries) = self.systems.get_mut(&phase) {
                        if let Some(entry) = entries.get_mut(i) {
                            entry.runner.run(world, &mut commands);
                        }
                    }
                }
                MixedNode::Script(i) => {
                    let desc = script_descs.get(i).expect("index from same slice");
                    script.run_script_descriptor(desc, world, host)?;
                }
            }
        }

        commands.apply(world);
        script.apply_commands(world)?;
        Ok(())
    }

    /// 仿真步内按契约顺序驱动统一调度图。
    pub fn run_sim_step(
        &mut self,
        world: &mut World,
        script: &mut SparkScriptDomain,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        self.run_rust_only(RustPhase::PreUpdate, world);

        for &phase in &[RustPhase::FixedUpdate, RustPhase::Update, RustPhase::LateUpdate] {
            self.run_mixed_phase(phase, world, script, host)?;
        }

        script.dispatch_events(host)?;
        Ok(())
    }

    /// 视觉帧渲染相位：混排 Rust 准备与 Spark Script 准备。
    pub fn run_render_frame(
        &mut self,
        world: &mut World,
        script: &mut SparkScriptDomain,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        if is_mixed_rust_phase(RustPhase::RenderPrepare) {
            self.run_mixed_phase(RustPhase::RenderPrepare, world, script, host)?;
        }
        self.run_rust_only(RustPhase::UiPrepare, world);
        Ok(())
    }
}
