//! 自 `src/query_view.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

use spark_engine::{command_apply::apply_script_commands, command_buffer::ScriptCommand};
use std::sync::Arc;

use spark_ecs::World;
use std::collections::HashSet;
#[test]
fn query_filters_by_archetype_name() {
    let mut world = World::new();
    apply_script_commands(
        &mut world,
        &[
            ScriptCommand::Spawn { archetype: Arc::from("rock") },
            ScriptCommand::Spawn { archetype: Arc::from("tree") },
            ScriptCommand::Spawn { archetype: Arc::from("rock") },
        ],
    )
    .unwrap();
    let view = ScriptQueryView::new(&world);
    assert_eq!(view.entities_with_archetype("rock").len(), 2);
    assert_eq!(view.entities_with_archetype("tree").len(), 1);
    assert!(view.entities_with_archetype("missing").is_empty());
}

#[test]
fn snapshot_exposes_count_and_entity_at() {
    let mut world = World::new();
    apply_script_commands(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("rock") }, ScriptCommand::Spawn { archetype: Arc::from("rock") }],
    )
    .unwrap();
    let snap = ScriptQuerySnapshot::from_world(&world);
    assert_eq!(snap.count("rock"), 2);
    assert_eq!(snap.count("missing"), 0);
    assert!(snap.entity_at("rock", 0).is_some());
    assert!(snap.entity_at("rock", 2).is_none());
}

#[test]
fn filtered_snapshot_hides_other_archetypes() {
    let mut world = World::new();
    apply_script_commands(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("rock") }, ScriptCommand::Spawn { archetype: Arc::from("tree") }],
    )
    .unwrap();
    let full = ScriptQuerySnapshot::from_world(&world);
    let allow: HashSet<Arc<str>> = [Arc::from("rock")].into_iter().collect();
    let view = full.filtered_by_archetypes(&allow);
    assert_eq!(view.count("rock"), 1);
    assert_eq!(view.count("tree"), 0);
    assert!(!view.contains_archetype("tree"));
}
