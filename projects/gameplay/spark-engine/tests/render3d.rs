//! 自 `src/render3d.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

use spark_renderer::Mat4;

use spark_types::Color;
use spark_ecs::World;
use spark_renderer::DrawList3d;
#[test]
fn systems_run_in_order() {
    let mut world = World::new();
    world.resources.insert(0u8);
    let mut schedule = RenderSchedule3d::new();
    schedule.add_fn("a", |w, _, _| {
        *w.resources.get_mut::<u8>().unwrap() += 1;
    });
    schedule.add_fn("b", |w, _, draw| {
        let n = *w.resources.get::<u8>().unwrap();
        draw.clear = Color::rgb(n as f32, 0.0, 0.0);
    });
    let mut draw = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
    schedule.draw(&mut world, &RenderFrame3d { screen_w: 1.0, screen_h: 1.0, clear: Color::rgb(0.0, 0.0, 0.0) }, &mut draw);
    assert!((draw.clear.r - 1.0).abs() < 1e-5);
}
