//! 自 `src/render2d.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

use spark_ecs::World;
use spark_renderer::DrawList;
use spark_types::Color;
#[test]
fn systems_run_in_order() {
    let mut world = World::new();
    world.resources.insert(0u32);
    let mut schedule = RenderSchedule2d::new();
    schedule.add_fn("a", |w, _, draw| {
        *w.resources.get_mut::<u32>().unwrap() += 1;
        draw.clear = Color::rgb(0.1, 0.0, 0.0);
    });
    schedule.add_fn("b", |w, frame, draw| {
        assert_eq!(*w.resources.get::<u32>().unwrap(), 1);
        assert!((frame.screen_w - 32.0).abs() < 1e-5);
        draw.clear = Color::rgb(0.2, 0.0, 0.0);
    });
    let frame = RenderFrame2d { screen_w: 32.0, screen_h: 16.0, clear: Color::rgb(0.0, 0.0, 0.0) };
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    schedule.draw(&mut world, &frame, &mut draw);
    assert!((draw.clear.r - 0.2).abs() < 1e-5);
}
