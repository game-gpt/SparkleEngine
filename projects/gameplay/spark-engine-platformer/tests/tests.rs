//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_types::{Rect, Vec2};
use spark_engine_platformer::*;

#[test]
fn land_and_jump() {
    let mut eng = PlatformerEngine::new(".");
    eng.world.push(SolidRect { rect: Rect::new(0.0, 0.0, 20.0, 1.0), kind: SolidKind::Solid });
    eng.player.pos = Vec2::new(2.0, 3.0);
    eng.player.vel = Vec2::new(0.0, -1.0);
    for _ in 0..30 {
        eng.tick(1.0 / 60.0, ControllerInput::default());
    }
    assert!(eng.player.on_ground);
    let y0 = eng.player.pos.y;
    eng.tick(1.0 / 60.0, ControllerInput { move_x: 0.0, jump_pressed: true, jump_held: true });
    assert!(eng.player.vel.y > 0.0 || eng.player.pos.y > y0);
}
