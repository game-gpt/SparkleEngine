//! �?`src/app3d.rs` 迁出的原 `#[cfg(test)] mod tests`�?use spark_engine::*;

use spark_types::Color;
use spark_input::Input;
use spark_renderer::{FrameCtx, GameHost3d, Mat4};

use spark_renderer::DrawList3d;
struct Paint;

impl SparkPlugin3d for Paint {
    fn build(&self, app: &mut SparkApp3d) {
        app.insert_resource(1u8);
        app.add_system("tick", |w| {
            *w.resources.get_mut::<u8>().unwrap() += 1;
        });
        app.add_render_fn("paint", |w, _, draw| {
            let n = *w.resources.get::<u8>().unwrap();
            draw.clear = Color::rgb(n as f32, 0.0, 0.0);
        });
    }
}

#[test]
fn plugin_builds_3d_host() {
    let mut app = SparkApp3d::new();
    app.add_plugin(&Paint);
    let mut host = app.into_host();
    let input = Input::default();
    host.update(&FrameCtx { input: &input, dt: 0.016, screen_w: 800.0, screen_h: 600.0, dpi_scale: 1.0, timing: Default::default() });
    let mut draw = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
    host.draw(&mut draw);
    assert!((draw.clear.r - 2.0).abs() < 1e-5);
}
