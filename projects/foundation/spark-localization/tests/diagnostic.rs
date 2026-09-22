//! 自 `src/diagnostic.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::*;

#[test]
fn flags_union() {
    let flags = DiagnosticFlags::MISSING.union(DiagnosticFlags::FALLBACK_USED);
    assert!(flags.contains(DiagnosticFlags::MISSING));
    assert!(flags.contains(DiagnosticFlags::FALLBACK_USED));
    assert!(!flags.contains(DiagnosticFlags::CYCLE));
}
