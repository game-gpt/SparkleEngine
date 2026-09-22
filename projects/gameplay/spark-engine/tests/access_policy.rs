//! 自 `src/access_policy.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;
use spark_script::{DeterminismClass, HostEffect, HostFunction, HostFunctionId, HostPhase, HostSchema};
#[test]
fn declared_empty_write_set_blocks_component_mutation() {
    let desc = ScriptSystemDescriptor::new("m", "s", "update", HostPhase::Update).read("Transform");
    let policy = ScriptAccessPolicy::from_descriptor(&desc);
    assert!(!policy.allows_write("Transform"));
    assert!(!policy.allows_write("Health"));
    assert!(policy.allows_read_world());
}

#[test]
fn declared_empty_access_blocks_world_query() {
    let desc = ScriptSystemDescriptor::new("m", "s", "update", HostPhase::Update);
    let policy = ScriptAccessPolicy::from_descriptor(&desc);
    assert!(!policy.allows_read_world());
}

#[test]
fn query_archetype_filter_from_descriptor() {
    let desc = ScriptSystemDescriptor::new("m", "s", "update", HostPhase::Update).read("Transform").query_archetype("rock");
    let policy = ScriptAccessPolicy::from_descriptor(&desc);
    assert!(policy.allows_query_archetype("rock"));
    assert!(!policy.allows_query_archetype("tree"));
}

#[test]
fn check_host_phase_denies_wrong_phase() {
    let mut schema = HostSchema::new(1);
    schema
        .insert(HostFunction::new(HostFunctionId::new("engine", "queue_spawn", 1)).phases([HostPhase::Update]).effect(HostEffect::SpawnEntity));
    assert!(check_host_phase(&schema, "queue_spawn", HostPhase::Update).is_ok());
    assert!(check_host_phase(&schema, "engine.queue_spawn", HostPhase::Update).is_ok());
    let err = check_host_phase(&schema, "queue_spawn", HostPhase::RenderPrepare).unwrap_err();
    assert!(err.contains("host_phase_denied"), "{err}");
    let unknown = check_host_phase(&schema, "no_such_host", HostPhase::Update).unwrap_err();
    assert!(unknown.contains("host_unknown:"), "{unknown}");
}

#[test]
fn check_host_determinism_denies_nondeterministic() {
    let mut schema = HostSchema::new(1);
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "log", 1)).determinism(DeterminismClass::Nondeterministic));
    let err = check_host_determinism(&schema, "engine.log", DeterminismClass::Deterministic).unwrap_err();
    assert!(err.contains("host_determinism_denied"), "{err}");
}
