//! �?`src/app.rs` 迁出的原 `#[cfg(test)] mod tests`�?use spark_engine::*;

use spark_types::Color;
use spark_input::Input;
use spark_renderer::{FrameCtx, GameHost};

use spark_renderer::DrawList;
struct Paint;

impl SparkPlugin for Paint {
    fn build(&self, app: &mut SparkApp) {
        app.insert_resource(7u8);
        app.add_system("tick", |w| {
            *w.resources.get_mut::<u8>().unwrap() += 1;
        });
        app.add_render_fn("paint", |w, _, draw| {
            let n = *w.resources.get::<u8>().unwrap();
            draw.clear = Color::rgb(n as f32 / 255.0, 0.0, 0.0);
        });
    }
}

#[test]
fn plugin_builds_host_that_ticks_and_paints() {
    let mut app = SparkApp::new();
    app.add_plugin(&Paint);
    let mut host = app.into_host();
    let input = Input::default();
    let frame = FrameCtx { input: &input, dt: 0.016, screen_w: 8.0, screen_h: 8.0, dpi_scale: 1.0, timing: Default::default() };
    host.update(&frame);
    let mut draw = spark_renderer::DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    host.draw(&mut draw);
    assert_eq!(*host.world().resources.get::<u8>().unwrap(), 8);
    assert!((draw.clear.r - 8.0 / 255.0).abs() < 1e-5);
}
