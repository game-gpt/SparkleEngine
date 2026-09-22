//! 自 `src/code.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_diagnostics::*;

#[test]
fn dotted_roundtrip() {
    let c = ErrorCode::parse("spark.asset.not_found");
    assert_eq!(c.namespace.as_str(), "spark");
    assert_eq!(c.id.as_str(), "asset.not_found");
    assert_eq!(c.to_string(), "spark.asset.not_found");
}
