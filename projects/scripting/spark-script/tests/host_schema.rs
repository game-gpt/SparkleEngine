//! 自 `src/host_schema.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;

use spark_script::{CompilationRequest, ScriptCompiler, ScriptLanguage};

#[test]
fn schema_slots_and_hash_are_stable() {
    let mut schema = HostSchema::new(1);
    schema.insert(
        HostFunction::new(HostFunctionId::new("spark.ecs", "spawn", 1))
            .param(NativeParam::new("archetype", "ArchetypeHandle"))
            .returns("PendingEntity")
            .effect(HostEffect::SpawnEntity)
            .capability("ecs.command")
            .phases([HostPhase::FixedUpdate, HostPhase::Update]),
    );
    schema.insert(
        HostFunction::new(HostFunctionId::new("spark.log", "print", 1))
            .param(NativeParam::new("msg", "String"))
            .returns("Null")
            .effect(HostEffect::Nondeterministic)
            .determinism(DeterminismClass::Nondeterministic),
    );

    let spawn_id = HostFunctionId::new("spark.ecs", "spawn", 1);
    assert_eq!(schema.slot_of(&spawn_id), Some(0));
    assert_eq!(schema.dispatch_names(), vec!["spark.ecs.spawn".to_string(), "spark.log.print".to_string()]);
    let binds = schema.to_bind_table().unwrap();
    assert_eq!(binds.len(), 2);
    assert_eq!(binds.resolve("spawn").unwrap().id.namespace.as_ref(), "spark.ecs");
    let h1 = schema.content_hash();
    let h2 = schema.content_hash();
    assert_eq!(h1, h2);
}

#[test]
fn short_name_conflict_rejected_by_bind_table() {
    let mut schema = HostSchema::new(1);
    schema.insert(HostFunction::new(HostFunctionId::new("graphics", "draw", 1)));
    schema.insert(HostFunction::new(HostFunctionId::new("ui", "draw", 1)));
    let err = schema.to_bind_table().unwrap_err();
    assert!(err.contains("host_short_name_conflict"), "{err}");
    assert!(schema.get_by_short_name("draw").is_none());
    assert!(schema.resolve_short_name("draw").unwrap_err().contains("conflict"));
}

#[test]
fn compile_rejects_missing_capability() {
    let mut host = HostSchema::new(1);
    host.insert(
        HostFunction::new(HostFunctionId::new("ecs", "spawn", 1))
            .capability("ecs.command")
            .effect(HostEffect::SpawnEntity)
            .determinism(DeterminismClass::Deterministic),
    );
    let mut request = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return spawn()", host);
    request.required_capabilities = vec![CapabilityId::new("other")];
    request.determinism = DeterminismClass::Deterministic;
    let err = ScriptCompiler::new().compile(&request).unwrap_err();
    assert!(format!("{err:?}").contains("capability") || err.code().contains("compile"), "{err:?}");
}

#[test]
fn allows_phase_respects_function_and_caller() {
    let f = HostFunction::new(HostFunctionId::new("ecs", "spawn", 1)).phases([HostPhase::FixedUpdate, HostPhase::Update]);
    assert!(f.allows_phase(HostPhase::Update));
    assert!(!f.allows_phase(HostPhase::RenderPrepare));
    assert!(f.allows_phase(HostPhase::Any));
    let open = HostFunction::new(HostFunctionId::new("log", "print", 1));
    assert!(open.allows_phase(HostPhase::RenderPrepare));
}

#[test]
fn bind_table_carries_effects_and_types() {
    let mut schema = HostSchema::new(1);
    schema.insert(
        HostFunction::new(HostFunctionId::new("ecs", "spawn", 1))
            .param(NativeParam::new("archetype", "ArchetypeHandle"))
            .returns("PendingEntity")
            .effect(HostEffect::SpawnEntity)
            .capability("ecs.command"),
    );
    let binds = schema.to_bind_table().unwrap();
    let e = binds.resolve("spawn").unwrap();
    assert_eq!(e.param_count, 1);
    assert_eq!(e.param_tys[0].as_ref(), "ArchetypeHandle");
    assert_eq!(e.return_ty.as_deref(), Some("PendingEntity"));
    assert!(e.effects.contains(&HostEffectKind::SpawnEntity));
    assert_eq!(e.required_capabilities[0].as_ref(), "ecs.command");
}
