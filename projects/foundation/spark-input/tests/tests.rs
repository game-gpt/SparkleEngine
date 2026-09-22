//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_input::*;

#[test]
fn ime_preedit_survives_begin_frame_until_commit() {
    let mut input = Input::default();
    input.on_ime_preedit("ni", Some((0, 2)));
    assert_eq!(input.composition(), "ni");
    input.begin_frame();
    assert_eq!(input.composition(), "ni");
    input.on_ime_commit("你");
    assert!(input.composition().is_empty());
    assert_eq!(input.text(), "你");
}
