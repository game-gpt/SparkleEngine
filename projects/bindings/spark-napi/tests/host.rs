//! 自 `src/host.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_napi::*;

#[test]
fn info_and_geometry() {
    let host = SparkJsHost::new();
    let info = host.info();
    assert_eq!(info.npm_package, "@game-gpt/sparkle-engine");
    assert!((host.vec2_length(3.0, 4.0) - 5.0).abs() < 1e-5);
}
