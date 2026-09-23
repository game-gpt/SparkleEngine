//! QueryPlan 绑定与 ScriptColumnBatch 列视图。

use std::sync::Arc;

use spark_ecs::World;
use spark_engine::{
    QueryPlan, ScriptColumnBatch, ScriptComponentCatalog, ScriptQuerySnapshot, ScriptSystemDescriptor,
    command_apply::{SCRIPT_MARKER_NAME, apply_script_commands},
    command_buffer::ScriptCommand,
};
use spark_script::HostPhase;

#[test]
fn query_plan_binds_component_slots() {
    let catalog = ScriptComponentCatalog::with_builtins();
    let desc = ScriptSystemDescriptor::new("m", "move", "update", HostPhase::Update)
        .read(SCRIPT_MARKER_NAME)
        .query_archetype("rock");
    let plan = QueryPlan::bind(&desc, &catalog).unwrap();
    assert_eq!(plan.columns().len(), 1);
    assert!(!plan.columns()[0].write);
    assert_eq!(plan.archetypes().len(), 1);
    assert_eq!(plan.archetypes()[0].as_ref(), "rock");
}

#[test]
fn query_plan_rejects_unknown_component() {
    let catalog = ScriptComponentCatalog::with_builtins();
    let desc = ScriptSystemDescriptor::new("m", "bad", "update", HostPhase::Update).write("NoSuchComponent");
    let err = QueryPlan::bind(&desc, &catalog).unwrap_err();
    assert!(err.to_string().contains("NoSuchComponent"));
}

#[test]
fn column_batch_entity_count_by_archetype_index() {
    let mut world = World::new();
    apply_script_commands(
        &mut world,
        &[
            ScriptCommand::Spawn { archetype: Arc::from("rock") },
            ScriptCommand::Spawn { archetype: Arc::from("rock") },
            ScriptCommand::Spawn { archetype: Arc::from("tree") },
        ],
    )
    .unwrap();
    let snapshot = ScriptQuerySnapshot::from_world(&world);
    let desc = ScriptSystemDescriptor::new("m", "sim", "update", HostPhase::Update).query_archetype("rock");
    let plan = QueryPlan::bind(&desc, &ScriptComponentCatalog::with_builtins()).unwrap();
    let batch = ScriptColumnBatch::install(plan, snapshot);
    assert_eq!(batch.entity_count(0), 2);
    assert_eq!(batch.views()[0].entities().len(), 2);
    assert_eq!(batch.total_entity_count(), 2);
}

#[test]
fn column_batch_unfiltered_installs_all_archetypes() {
    let mut world = World::new();
    apply_script_commands(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("rock") }, ScriptCommand::Spawn { archetype: Arc::from("tree") }],
    )
    .unwrap();
    let snapshot = ScriptQuerySnapshot::from_world(&world);
    let desc = ScriptSystemDescriptor::new("m", "sim", "update", HostPhase::Update);
    let plan = QueryPlan::bind(&desc, &ScriptComponentCatalog::with_builtins()).unwrap();
    assert!(plan.is_unfiltered());
    let batch = ScriptColumnBatch::install(plan, snapshot);
    assert_eq!(batch.views().len(), 2);
    assert_eq!(batch.total_entity_count(), 2);
}
