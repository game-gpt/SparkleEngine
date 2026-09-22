//! 自 `src/cache.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;

use spark_script::{CompilationRequest, HostSchema, ScriptLanguage};

#[test]
fn same_request_same_key() {
    let host = HostSchema::new(1);
    let a = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1", host.clone());
    let b = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1", host);
    assert_eq!(ArtifactCache::key_for(&a, "return 1"), ArtifactCache::key_for(&b, "return 1"));
}

#[test]
fn source_change_changes_key() {
    let host = HostSchema::new(1);
    let req = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1", host);
    assert_ne!(ArtifactCache::key_for(&req, "return 1"), ArtifactCache::key_for(&req, "return 2"));
}

#[test]
fn optimization_change_changes_key() {
    use spark_script::OptimizationLevel;
    let host = HostSchema::new(1);
    let mut a = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1", host.clone());
    let mut b = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1", host);
    a.optimization = OptimizationLevel::None;
    b.optimization = OptimizationLevel::Aggressive;
    assert_ne!(ArtifactCache::key_for(&a, "return 1"), ArtifactCache::key_for(&b, "return 1"));
}

#[test]
fn determinism_change_changes_key() {
    use spark_script::DeterminismClass;
    let host = HostSchema::new(1);
    let mut a = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1", host.clone());
    let mut b = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1", host);
    a.determinism = DeterminismClass::Deterministic;
    b.determinism = DeterminismClass::Nondeterministic;
    assert_ne!(ArtifactCache::key_for(&a, "return 1"), ArtifactCache::key_for(&b, "return 1"));
}
