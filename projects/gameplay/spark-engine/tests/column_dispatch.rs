//! 列直写 dispatch 原生（query_column_f32 / set_column_f32）。

use std::sync::Arc;

use spark_ecs::World;
use spark_engine::{
    ScriptComponentLayoutBuilder, ScriptComponentStore, ScriptFieldKind, ScriptSystemDescriptor, SparkEngine,
    apply_script_commands_with_store, command_buffer::ScriptCommand,
};
use spark_script::HostPhase;
use spark_vm::StdHost;

fn write_column_mod(mod_id: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("spark_column_{mod_id}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        format!("id = \"{mod_id}\"\nversion = \"0.0.0\"\nentry = \"main.vk\"\n"),
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro update() {
            set_column_f32(0, 0, 0, 0, 99.0)
            return query_column_f32(0, 0, 0, 0)
        }
        return 0
        "#,
    )
    .unwrap();
    root
}

#[test]
fn column_f32_write_and_read_via_script() {
    let mod_id = "column_dispatch";
    let root = write_column_mod(mod_id);
    let mut eng = SparkEngine::new(root.parent().unwrap());
    let layout = ScriptComponentLayoutBuilder::new().field("hp", ScriptFieldKind::F32).build().unwrap();
    eng.register_script_component_layout("Health", layout);
    eng.load_mod_dir(&root).unwrap();
    let desc = ScriptSystemDescriptor::new(mod_id, "update", "update", HostPhase::Update)
        .write("Health")
        .query_archetype("rock");
    eng.register_script_system(desc).unwrap();

    let mut world = World::new();
    let catalog = eng.component_catalog().clone();
    world.resources.insert(ScriptComponentStore::new());
    let mut store = world.resources.get::<ScriptComponentStore>().cloned();
    let spawn = apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("rock") }],
        &catalog,
        &mut store,
    )
    .unwrap();
    let entity = spawn.spawned[0].to_bits();
    apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::AddComponent { entity, component: Arc::from("Health") }],
        &catalog,
        &mut store,
    )
    .unwrap();

    eng.set_execution_profile(spark_engine::ExecutionProfile::Trusted);
    let mut host = StdHost;
    let value = eng.run_script_systems(HostPhase::Update, &mut world, &mut host).unwrap();
    assert_eq!(value.spawned.len(), 0);

    let store = world.resources.get::<ScriptComponentStore>().unwrap();
    let slot = catalog.id_of("Health").unwrap();
    let layout = catalog.layout_of(slot).unwrap();
    assert_eq!(store.read_f32(slot, layout, entity, 0), Some(99.0));
}

fn write_stats_mod(mod_id: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("spark_column_{mod_id}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        format!("id = \"{mod_id}\"\nversion = \"0.0.0\"\nentry = \"main.vk\"\n"),
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro update() {
            set_column_i32(0, 0, 0, 0, -7)
            set_column_bool(0, 0, 0, 1, 1)
            return query_column_i32(0, 0, 0, 0)
        }
        return 0
        "#,
    )
    .unwrap();
    root
}

#[test]
fn column_i32_and_bool_write_and_read_via_script() {
    let mod_id = "column_i32_bool";
    let root = write_stats_mod(mod_id);
    let mut eng = SparkEngine::new(root.parent().unwrap());
    let layout = ScriptComponentLayoutBuilder::new()
        .field("score", ScriptFieldKind::I32)
        .field("alive", ScriptFieldKind::Bool)
        .build()
        .unwrap();
    eng.register_script_component_layout("Stats", layout);
    eng.load_mod_dir(&root).unwrap();
    let desc = ScriptSystemDescriptor::new(mod_id, "update", "update", HostPhase::Update)
        .write("Stats")
        .query_archetype("rock");
    eng.register_script_system(desc).unwrap();

    let mut world = World::new();
    let catalog = eng.component_catalog().clone();
    world.resources.insert(ScriptComponentStore::new());
    let mut store = world.resources.get::<ScriptComponentStore>().cloned();
    let spawn = apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("rock") }],
        &catalog,
        &mut store,
    )
    .unwrap();
    let entity = spawn.spawned[0].to_bits();
    apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::AddComponent { entity, component: Arc::from("Stats") }],
        &catalog,
        &mut store,
    )
    .unwrap();

    eng.set_execution_profile(spark_engine::ExecutionProfile::Trusted);
    let mut host = StdHost;
    eng.run_script_systems(HostPhase::Update, &mut world, &mut host).unwrap();

    let store = world.resources.get::<ScriptComponentStore>().unwrap();
    let slot = catalog.id_of("Stats").unwrap();
    let layout = catalog.layout_of(slot).unwrap();
    assert_eq!(store.read_i32(slot, layout, entity, 0), Some(-7));
    assert_eq!(store.read_bool(slot, layout, entity, 1), Some(true));
}
