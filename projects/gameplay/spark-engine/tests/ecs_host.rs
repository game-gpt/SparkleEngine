//! �?`src/ecs_host.rs` 迁出的原 `#[cfg(test)] mod tests`�?use spark_engine::*;

use spark_types::Color;
use spark_ecs::{Schedule, World};
use spark_input::Input;
use spark_renderer::{DrawList, DrawList3d, FrameCtx, GameHost, GameHost3d, Mat4, WindowConfig};
fn frame_ctx(input: &Input) -> FrameCtx<'_> {
    FrameCtx { input, dt: 1.0 / 60.0, screen_w: 1280.0, screen_h: 720.0, dpi_scale: 1.0, timing: Default::default() }
}

#[test]
fn host_2d_schedule_fills_draw_buffer() {
    let mut world = World::new();
    world.resources.insert(DrawBuffer2d::default());
    let mut schedule = Schedule::new();
    schedule.add_fn("draw", |w| {
        let list = DrawList::new(Color::rgb(0.2, 0.3, 0.4));
        w.resources.get_mut::<DrawBuffer2d>().unwrap().list = Some(list);
    });
    // 仿真 schedule 空；绘制�?draw_schedule
    let mut host = EcsHost2d::new(world, Schedule::new()).with_draw_schedule(schedule);
    let input = Input::default();
    host.update(&frame_ctx(&input));
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    host.draw(&mut draw);
    assert!((draw.clear.r - 0.2).abs() < 1e-5);
}

#[test]
fn host_2d_fallback_mutates_world() {
    #[derive(Default)]
    struct Marker(u32);
    let mut world = World::new();
    world.resources.insert(Marker(0));
    let mut host = EcsHost2d::new(world, Schedule::new()).with_draw_fallback(|w, draw| {
        w.resources.get_mut::<Marker>().unwrap().0 += 1;
        draw.clear = Color::rgb(0.5, 0.0, 0.0);
    });
    let input = Input::default();
    host.update(&frame_ctx(&input));
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    host.draw(&mut draw);
    assert_eq!(host.world.resources.get::<Marker>().unwrap().0, 1);
    assert!((draw.clear.r - 0.5).abs() < 1e-5);
}

#[test]
fn host_2d_honours_app_exit() {
    let mut world = World::new();
    world.resources.insert(AppExit { requested: true });
    let host = EcsHost2d::new(world, Schedule::new());
    assert!(host.should_exit());
}

#[test]
fn schedule_fills_draw_buffer_3d() {
    let mut world = World::new();
    world.resources.insert(DrawBuffer3d::default());
    let mut schedule = Schedule::new();
    schedule.add_fn("draw", |w| {
        let list = DrawList3d::new(Color::rgb(0.1, 0.2, 0.3), Mat4::IDENTITY);
        w.resources.get_mut::<DrawBuffer3d>().unwrap().list = Some(list);
    });
    let mut host = EcsHost3d::new(world, schedule);
    let input = Input::default();
    host.update(&frame_ctx(&input));
    let mut draw = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
    host.draw(&mut draw);
    assert!((draw.clear.r - 0.1).abs() < 1e-5);
    let _ = WindowConfig::default();
}
