//! ScriptApiProvider 装配与禁用。

use spark_engine::{CoreEngineScriptApiProvider, ScriptApiProvider, ScriptApiRegistry, SparkEngine, engine_host_schema};
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
fn disabled_spawn_blocks_mod_compile_at_load() {
    let root = std::env::temp_dir().join("spark_api_disable_spawn");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "no_spawn"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            queue_spawn("rock")
            return 0
        }
        return 0
        "#,
    )
    .unwrap();

    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.api_registry_mut().disable("engine.queue_spawn");
    let err = eng.load_mod_dir(&root).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("host") || msg.contains("spawn") || msg.contains("compile") || msg.contains("Script"),
        "{msg}"
    );
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
