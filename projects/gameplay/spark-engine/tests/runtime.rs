//! SparkRuntime 双域调度与场景切换。

use spark_engine::*;
use spark_input::Input;
use spark_renderer::{DrawList, FrameCtx, WindowPump2d};
use spark_types::Color;
use spark_vm::StdHost;

fn frame_ctx(input: &Input, dt: f32) -> FrameCtx<'_> {
    FrameCtx { input, dt, screen_w: 640.0, screen_h: 480.0, dpi_scale: 1.0, timing: Default::default() }
}

struct CounterPlugin;

impl NativeGamePlugin for CounterPlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.insert_resource(0u32);
        runtime.scenes_mut().on_enter("main", |world| {
            *world.resources.get_mut::<u32>().unwrap() = 1;
        });
        runtime.load_scene("main");
        runtime.add_system_ctx(RustPhase::Update, "inc", |ctx| {
            *ctx.world.resources.get_mut::<u32>().unwrap() += 1;
        });
        runtime.add_render_fn("paint", |w, _, draw| {
            let n = *w.resources.get::<u32>().unwrap();
            draw.clear = Color::rgb(n as f32 / 255.0, 0.0, 0.0);
        });
    }
}

#[test]
fn runtime_ticks_and_renders_via_window_pump() {
    let mut runtime = SparkRuntime::new();
    runtime.register_native(&CounterPlugin);
    let mut pump = runtime.into_host();
    let input = Input::default();
    let frame = frame_ctx(&input, 1.0 / 60.0);
    pump.simulate(&frame);
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    pump.present_world(&mut draw);
    assert_eq!(*pump.runtime.world().resources.get::<u32>().unwrap(), 2);
    assert!((draw.clear.r - 2.0 / 255.0).abs() < 1e-5);
}

#[test]
fn runtime_scene_transition_via_command() {
    let mut runtime = SparkRuntime::new();
    runtime.insert_resource(String::from("none"));
    runtime.scenes_mut().on_enter("a", |world| {
        *world.resources.get_mut::<String>().unwrap() = "a".into();
    });
    runtime.scenes_mut().on_enter("b", |world| {
        *world.resources.get_mut::<String>().unwrap() = "b".into();
    });
    runtime.load_scene("a");
    let input = Input::default();
    let frame = frame_ctx(&input, 0.016);
    let mut host = StdHost;
    runtime.tick_sim(&frame, &mut host).unwrap();
    assert_eq!(runtime.world().resources.get::<String>().unwrap().as_str(), "a");
    runtime.enqueue_scene_command(SceneCommand::Transition { to: "b".into() });
    runtime.tick_sim(&frame, &mut host).unwrap();
    assert_eq!(runtime.world().resources.get::<String>().unwrap().as_str(), "b");
}

#[test]
fn runtime_exit_resource() {
    let mut runtime = SparkRuntime::new();
    runtime.world_mut().resources.get_mut::<AppExit>().unwrap().request();
    assert!(runtime.should_exit());
}
