//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_wasm::*;

#[test]
fn exports_geometry() {
    let len = spark_vec2_length(3.0, 4.0);
    assert!((len - 5.0).abs() < 1e-9);
    assert_eq!(spark_version_code(), 0);
}
