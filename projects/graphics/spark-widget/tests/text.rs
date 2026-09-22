//! 自 `src/text/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

#[test]
fn estimate_wraps_when_max_width_set() {
    let style = TextStyle { size: 10.0, ..TextStyle::default() };
    let layout = measure_plain("abcdefghij", &style, Some(30.0));
    assert!(layout.line_count >= 2);
}
