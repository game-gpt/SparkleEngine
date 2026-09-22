//! 自 `src/script_system.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;
use spark_script::HostPhase;
use std::sync::Arc;

#[test]
fn registry_filters_by_phase() {
    let mut reg = ScriptSystemRegistry::new();
    reg.register(ScriptSystemDescriptor::new("demo", "move", "fixed_update", HostPhase::FixedUpdate));
    reg.register(ScriptSystemDescriptor::new("demo", "draw", "render_prepare", HostPhase::RenderPrepare).read("Transform"));
    assert_eq!(reg.for_phase(HostPhase::FixedUpdate).count(), 1);
    assert_eq!(reg.for_phase(HostPhase::RenderPrepare).count(), 1);
    assert_eq!(reg.for_phase(HostPhase::Update).count(), 0);
}

#[test]
fn lifecycle_exports_register_phases() {
    let mut reg = ScriptSystemRegistry::new();
    let exports = [Arc::<str>::from("on_load"), Arc::<str>::from("update"), Arc::<str>::from("helper")];
    reg.register_lifecycle_exports("m", &exports);
    assert_eq!(reg.len(), 2);
    assert!(reg.for_phase(HostPhase::Update).any(|s| s.entry.as_ref() == "update"));
}

#[test]
fn before_after_orders_systems() {
    let mut reg = ScriptSystemRegistry::new();
    reg.register(ScriptSystemDescriptor::new("m", "b", "b", HostPhase::Update).after("a"));
    reg.register(ScriptSystemDescriptor::new("m", "a", "a", HostPhase::Update).before("b"));
    let ordered = reg.ordered_for_phase(HostPhase::Update).unwrap();
    assert_eq!(ordered[0].name.as_ref(), "a");
    assert_eq!(ordered[1].name.as_ref(), "b");
}

#[test]
fn write_write_access_conflicts() {
    let mut reg = ScriptSystemRegistry::new();
    reg.register(ScriptSystemDescriptor::new("m", "a", "a", HostPhase::Update).write("Transform"));
    reg.register(ScriptSystemDescriptor::new("m", "b", "b", HostPhase::Update).write("Transform"));
    let err = reg.ordered_for_phase(HostPhase::Update).unwrap_err();
    assert!(matches!(err, ScriptSystemError::AccessConflict { .. }));
}

#[test]
fn cycle_is_rejected() {
    let mut reg = ScriptSystemRegistry::new();
    reg.register(ScriptSystemDescriptor::new("m", "a", "a", HostPhase::Update).before("b"));
    reg.register(ScriptSystemDescriptor::new("m", "b", "b", HostPhase::Update).before("a"));
    let err = reg.ordered_for_phase(HostPhase::Update).unwrap_err();
    assert!(matches!(err, ScriptSystemError::Cycle { .. }));
}
