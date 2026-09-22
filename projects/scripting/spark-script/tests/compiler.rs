//! 自 `src/compiler.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;

use spark_script::{CompilationRequest, ScriptLanguage};

#[test]
fn compile_hits_artifact_cache() {
    let host = HostSchema::new(1);
    let mut compiler = ScriptCompiler::new();
    let a = compiler.compile_source(ScriptLanguage::Valkyrie, "return 1 + 2", &host).unwrap();
    assert_eq!(compiler.cache.misses, 1);
    assert_eq!(compiler.cache.hits, 0);
    let b = compiler.compile_source(ScriptLanguage::Valkyrie, "return 1 + 2", &host).unwrap();
    assert_eq!(compiler.cache.hits, 1);
    assert_eq!(compiler.cache.len(), 1);
    assert_eq!(a.image.host_schema_hash, b.image.host_schema_hash);
}

#[test]
fn compile_object_and_link_roundtrip() {
    let host = HostSchema::new(1);
    let req = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1 + 2", host.clone());
    let mut compiler = ScriptCompiler::new();
    let obj = compiler.compile_object(&req).unwrap();
    let bytes = obj.to_spko_bytes().unwrap();
    let loaded = SparkObject::from_spko_bytes(&bytes).unwrap();
    let package = compiler.link_objects(&[loaded], &host, &req.package).unwrap();
    let mut rt = spark_script::ScriptRuntime::from_image(&package.image, &host).unwrap();
    let v = rt.call_on_load_std().unwrap();
    assert_eq!(v.as_number(), Some(3.0));
}
