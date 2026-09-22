//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_time::*;

#[test]
fn fixed_steps_accumulate() {
    let mut c = Clock::fixed(0.05, 8);
    assert_eq!(c.begin_frame(0.12), 2);
    assert!((c.elapsed_seconds - 0.10).abs() < 1e-5);
    assert_eq!(c.begin_frame(0.01), 0);
    assert_eq!(c.begin_frame(0.04), 1);
}

#[test]
fn pause_and_scale() {
    let mut c = Clock::variable();
    c.set_paused(true);
    assert_eq!(c.begin_frame(0.016), 0);
    c.set_paused(false);
    c.set_scale(2.0);
    assert_eq!(c.begin_frame(0.016), 1);
    assert!((c.delta_seconds - 0.032).abs() < 1e-5);
}
