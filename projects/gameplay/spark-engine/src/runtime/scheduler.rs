//! 双域调度器：Rust 相位表 + Spark Script 相位钩子。

use std::collections::HashMap;

use spark_ecs::{Schedule, World};
use spark_script::HostPhase;
use spark_vm::HostHooks;

use super::phase::RustPhase;
use super::script_domain::SparkScriptDomain;
use crate::EngineError;

/// 按相位分桶的 Rust 系统表与 Spark Script 桥。
pub struct RuntimeScheduler {
    rust: HashMap<RustPhase, Schedule>,
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
    pub fn add_rust_fn(&mut self, phase: RustPhase, name: &'static str, f: impl FnMut(&mut World) + Send + 'static) {
        self.rust.entry(phase).or_insert_with(Schedule::new).add_fn(name, f);
    }

    /// 运行单个 Rust 相位。
    pub fn run_rust(&mut self, phase: RustPhase, world: &mut World) {
        if let Some(schedule) = self.rust.get_mut(&phase) {
            schedule.run(world);
        }
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

    /// 按 [`RustPhase::SIM_ORDER`] 合并各相位调度表。
    pub fn into_flat_sim(mut self) -> Schedule {
        let mut flat = Schedule::new();
        for phase in RustPhase::SIM_ORDER {
            if let Some(sched) = self.rust.remove(&phase) {
                flat.merge(sched);
            }
        }
        flat
    }
}
