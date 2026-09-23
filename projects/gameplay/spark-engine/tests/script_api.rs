//! ScriptApiProvider 装配与禁用。

use spark_engine::{CoreEngineScriptApiProvider, ScriptApiProvider, ScriptApiRegistry, engine_host_schema};
use spark_script::{HostFunction, HostFunctionId, HostPhase, HostSchema};

struct TestGameApi;

impl ScriptApiProvider for TestGameApi {
    fn register(&self, schema: &mut HostSchema) {
        schema.insert(HostFunction::new(HostFunctionId::new("game", "ping", 1)).phases([HostPhase::Any]));
    }
}

#[test]
fn registry_applies_providers() {
    let mut registry = ScriptApiRegistry::new();
    registry.register_provider(Box::new(TestGameApi));
    let mut schema = engine_host_schema();
    registry.apply_to(&mut schema);
    assert!(schema.resolve_import("game.ping").is_ok());
}

#[test]
fn registry_disable_removes_host_function() {
    let mut registry = ScriptApiRegistry::new();
    registry.disable("engine.queue_spawn");
    let mut schema = engine_host_schema();
    registry.apply_to(&mut schema);
    assert!(schema.resolve_import("engine.queue_spawn").is_err());
    assert!(schema.resolve_import("queue_spawn").is_err());
}

#[test]
fn core_provider_is_noop() {
    let mut registry = ScriptApiRegistry::new();
    registry.register_provider(Box::new(CoreEngineScriptApiProvider));
    let mut schema = engine_host_schema();
    let before = schema.functions.len();
    registry.apply_to(&mut schema);
    assert_eq!(schema.functions.len(), before);
}
