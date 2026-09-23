//! 脚本组件列存储与命令提交集成。

use std::sync::Arc;

use spark_ecs::World;
use spark_engine::{
    ScriptComponentLayoutBuilder, ScriptComponentStore, ScriptFieldKind, apply_script_commands_with_store,
    command_buffer::ScriptCommand,
};

#[test]
fn layout_component_add_initializes_store_row() {
    let mut world = World::new();
    world.resources.insert(ScriptComponentStore::new());
    let mut catalog = spark_engine::ScriptComponentCatalog::with_builtins();
    let layout = ScriptComponentLayoutBuilder::new().field("hp", ScriptFieldKind::F32).build().unwrap();
    catalog.register_with_layout("Health", layout);
    let mut store = world.resources.get::<ScriptComponentStore>().cloned();
    let spawn = apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("unit") }],
        &catalog,
        &mut store,
    )
    .unwrap();
    let entity = spawn.spawned[0].to_bits();
    let report = apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::AddComponent { entity, component: Arc::from("Health") }],
        &catalog,
        &mut store,
    )
    .unwrap();
    assert_eq!(report.added_components, 1);
    let store = world.resources.get::<ScriptComponentStore>().unwrap();
    let slot = catalog.id_of("Health").unwrap();
    assert_eq!(store.read_f32(slot, catalog.layout_of(slot).unwrap(), entity, 0), Some(0.0));
}

#[test]
fn despawn_removes_all_component_rows() {
    let mut world = World::new();
    world.resources.insert(ScriptComponentStore::new());
    let mut catalog = spark_engine::ScriptComponentCatalog::with_builtins();
    let layout = ScriptComponentLayoutBuilder::new().field("hp", ScriptFieldKind::F32).build().unwrap();
    catalog.register_with_layout("Health", layout);
    let mut store = world.resources.get::<ScriptComponentStore>().cloned();
    let spawn = apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("unit") }],
        &catalog,
        &mut store,
    )
    .unwrap();
    let entity = spawn.spawned[0].to_bits();
    let slot = catalog.id_of("Health").unwrap();
    let layout = catalog.layout_of(slot).unwrap();
    store.as_mut().unwrap().write_f32(slot, layout, entity, 0, 42.0);
    apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::Despawn { entity }],
        &catalog,
        &mut store,
    )
    .unwrap();
    assert_eq!(store.as_ref().unwrap().read_f32(slot, layout, entity, 0), None);
}
