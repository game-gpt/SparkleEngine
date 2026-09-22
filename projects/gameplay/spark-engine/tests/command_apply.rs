//! 自 `src/command_apply.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

use std::sync::Arc;

use spark_ecs::World;
#[test]
fn spawn_and_despawn_apply() {
    let mut world = World::new();
    let report = apply_script_commands(&mut world, &[ScriptCommand::Spawn { archetype: Arc::from("rock") }]).unwrap();
    assert_eq!(report.spawned.len(), 1);
    let e = report.spawned[0];
    assert_eq!(world.get::<ScriptArchetypeTag>(e).map(|t| t.name.as_ref()), Some("rock"));
    let report2 = apply_script_commands(&mut world, &[ScriptCommand::Despawn { entity: e.to_bits() }]).unwrap();
    assert_eq!(report2.despawned, vec![e]);
    assert!(!world.is_alive(e));
}

#[test]
fn unknown_component_edits_fail() {
    let mut world = World::new();
    let err = apply_script_commands(&mut world, &[ScriptCommand::AddComponent { entity: 0, component: Arc::from("Health") }]).unwrap_err();
    assert!(matches!(
        err,
        CommandApplyError::UnknownComponent { ref component } if component.as_ref() == "Health"
    ));
}

#[test]
fn script_marker_add_and_remove() {
    let mut world = World::new();
    let spawn = apply_script_commands(&mut world, &[ScriptCommand::Spawn { archetype: Arc::from("unit") }]).unwrap();
    let e = spawn.spawned[0];
    let catalog = ScriptComponentCatalog::with_builtins();
    let added = apply_script_commands_with(
        &mut world,
        &[ScriptCommand::AddComponent { entity: e.to_bits(), component: Arc::from(SCRIPT_MARKER_NAME) }],
        &catalog,
    )
    .unwrap();
    assert_eq!(added.added_components, 1);
    assert!(world.get::<ScriptMarker>(e).is_some());
    let removed = apply_script_commands_with(
        &mut world,
        &[ScriptCommand::RemoveComponent { entity: e.to_bits(), component: Arc::from(SCRIPT_MARKER_NAME) }],
        &catalog,
    )
    .unwrap();
    assert_eq!(removed.removed_components, 1);
    assert!(world.get::<ScriptMarker>(e).is_none());
}
