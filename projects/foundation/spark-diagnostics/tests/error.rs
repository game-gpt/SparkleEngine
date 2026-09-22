//! 自 `src/error.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_diagnostics::*;

#[test]
fn display_is_stable_code_only() {
    let err = Error::not_implemented("widget-rtl");
    assert_eq!(err.to_string(), "spark.not_implemented");
    assert!(err.args.get("what").is_some());
}
