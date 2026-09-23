//! 统一调度图、RustCommands 与 SystemContext 查询助手。

use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime, SystemOrder};
use spark_input::Input;
use spark_renderer::FrameCtx;
use spark_vm::StdHost;

#[derive(Debug, Clone, Copy, Default)]
struct Counter {
    value: u32,
}

struct CounterPlugin;

impl NativeGamePlugin for CounterPlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.world_mut().spawn(Counter { value: 0 });
        runtime.add_system_ctx(RustPhase::Update, "inc_counter", |ctx| {
            ctx.query_mut::<Counter>(|_, c| c.value += 1);
        });
    }
}

fn frame_ctx(input: &Input, dt: f32) -> FrameCtx<'_> {
    FrameCtx { input, dt, screen_w: 640.0, screen_h: 480.0, dpi_scale: 1.0, timing: Default::default() }
}

#[test]
fn system_context_query_mut_increments_component() {
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
    runtime.add_system_ctx(RustPhase::Update, "spawn_then_count", |ctx| {
        if ctx.world.entity_count() == 0 {
            ctx.commands.spawn(Counter { value: 7 });
        }
    });
    runtime.add_system_ctx(RustPhase::Update, "read_before_apply", |ctx| {
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

#[test]
fn system_before_after_ordering() {
    let mut runtime = SparkRuntime::new();
    runtime.insert_resource(String::new());
    runtime.add_system_ctx(RustPhase::Update, "first", |ctx| {
        ctx.world.resources.get_mut::<String>().unwrap().push_str("a");
    });
    runtime.add_system_ctx_with_order(
        RustPhase::Update,
        "second",
        SystemOrder { after: vec!["first"], ..SystemOrder::default() },
        |ctx| {
            ctx.world.resources.get_mut::<String>().unwrap().push_str("b");
        },
    );
    let input = Input::default();
    let frame = frame_ctx(&input, 0.016);
    let mut host = StdHost;
    runtime.tick_sim(&frame, &mut host).unwrap();
    assert_eq!(runtime.world().resources.get::<String>().unwrap().as_str(), "ab");
}
