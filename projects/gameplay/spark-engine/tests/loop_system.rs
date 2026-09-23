//! LoopSystem、RustCommands 与 query 助手。

use spark_engine::{LoopSystem, NativeGamePlugin, RustPhase, SparkRuntime, SystemContext};
use spark_input::Input;
use spark_renderer::FrameCtx;
use spark_vm::StdHost;

#[derive(Debug, Clone, Copy)]
struct Counter {
    value: u32,
}

#[derive(Default)]
struct IncSystem {
    step: u32,
}

impl LoopSystem for IncSystem {
    fn name(&self) -> &'static str {
        "inc_counter"
    }

    fn run(&mut self, ctx: &mut SystemContext<'_>) {
        ctx.query_mut::<Counter>(|_, c| c.value += self.step);
    }
}

struct CounterPlugin;

impl NativeGamePlugin for CounterPlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.world_mut().spawn(Counter { value: 0 });
        runtime.add_loop_system(RustPhase::Update, IncSystem { step: 1 });
    }
}

fn frame_ctx(input: &Input, dt: f32) -> FrameCtx<'_> {
    FrameCtx { input, dt, screen_w: 640.0, screen_h: 480.0, dpi_scale: 1.0, timing: Default::default() }
}

#[test]
fn loop_system_query_mut_increments_component() {
    let mut runtime = SparkRuntime::new();
    runtime.register_native(&CounterPlugin);
    let input = Input::default();
    let frame = frame_ctx(&input, 1.0 / 60.0);
    let mut host = StdHost;
    runtime.tick_sim(&frame, &mut host).unwrap();
    runtime.tick_sim(&frame, &mut host).unwrap();
    let mut found = None;
    runtime.world().for_each::<Counter>(|_, c| found = Some(c.value));
    assert_eq!(found, Some(2));
}

#[test]
fn rust_commands_apply_at_phase_boundary() {
    let mut runtime = SparkRuntime::new();
    runtime.add_loop_system_fn(RustPhase::Update, "spawn_then_count", |ctx| {
        if ctx.world.entity_count() == 0 {
            ctx.commands.spawn(Counter { value: 7 });
        }
    });
    runtime.add_loop_system_fn(RustPhase::Update, "read_before_apply", |ctx| {
        assert_eq!(ctx.world.entity_count(), 0);
    });
    let input = Input::default();
    let frame = frame_ctx(&input, 0.016);
    let mut host = StdHost;
    runtime.tick_sim(&frame, &mut host).unwrap();
    assert_eq!(runtime.world().entity_count(), 1);
    let mut found = None;
    runtime.world().for_each::<Counter>(|_, c| found = Some(c.value));
    assert_eq!(found, Some(7));
}
