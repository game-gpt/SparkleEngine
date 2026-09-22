//! 自 `src/actions.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_input::*;

use spark_input::ButtonState;

#[test]
fn axis_and_edge_follow_bindings() {
    let mut map = ActionMap::new();
    map.bind_key("left", Key::A).bind_key("left", Key::Left);
    map.bind_key("right", Key::D);
    map.bind_mouse("use", MouseBtn::Left);

    let mut input = Input::default();
    input.on_key(Key::A, ButtonState::Pressed);
    input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
    assert!(map.down(&input, "left"));
    assert!(map.pressed(&input, "use"));
    assert!((map.axis(&input, "left", "right") + 1.0).abs() < 1e-5);

    input.begin_frame();
    input.on_key(Key::A, ButtonState::Released);
    assert!(map.released(&input, "left"));
    assert!(!map.down(&input, "left"));
}
