//! `game.apply_damage` / `game.is_alive` 原生与 `SparkRuntime` Provider 保留。

use std::sync::Arc;

use spark_ecs::World;
use spark_engine::{
    GameCombatScriptApiProvider, ScriptComponentLayoutBuilder, ScriptComponentStore, ScriptFieldKind, ScriptSystemDescriptor,
    SparkEngine, SparkRuntime, apply_script_commands_with_store, command_buffer::ScriptCommand,
};
use spark_script::HostPhase;
use spark_vm::StdHost;

fn combat_mod_in(parent: &std::path::Path, mod_id: &str) -> std::path::PathBuf {
    let root = parent.join(mod_id);
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
            let e = query_entity_at("fighter", 0)
            apply_damage(e, 15.0)
            return is_alive(e)
        }
        return 0
        "#,
    )
    .unwrap();
    root
}

fn isolated_mods_parent(label: &str) -> std::path::PathBuf {
    let parent = std::env::temp_dir().join(format!("spark_game_combat_{label}"));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    parent
}

#[test]
fn game_combat_natives_damage_and_alive() {
    let mod_id = "game_combat";
    let parent = isolated_mods_parent("damage");
    let root = combat_mod_in(&parent, mod_id);
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.api_registry_mut().register_provider(Box::new(GameCombatScriptApiProvider));
    let layout = ScriptComponentLayoutBuilder::new().field("hp", ScriptFieldKind::F32).build().unwrap();
    eng.register_script_component_layout("Health", layout);
    eng.load_mod_dir(&root).unwrap();
    let desc = ScriptSystemDescriptor::new(mod_id, "update", "update", HostPhase::Update)
        .write("Health")
        .read("Health")
        .query_archetype("fighter");
    eng.register_script_system(desc).unwrap();

    let mut world = World::new();
    let catalog = eng.component_catalog().clone();
    world.resources.insert(ScriptComponentStore::new());
    let mut store = world.resources.get::<ScriptComponentStore>().cloned();
    let spawn = apply_script_commands_with_store(
        &mut world,
        &[ScriptCommand::Spawn { archetype: Arc::from("fighter") }],
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
    if let Some(store) = store.as_mut() {
        let slot = catalog.id_of("Health").unwrap();
        let layout = catalog.layout_of(slot).unwrap();
        assert!(store.write_f32(slot, layout, entity, 0, 10.0));
    }
    if let Some(store) = store {
        world.resources.insert(store);
    }

    eng.set_execution_profile(spark_engine::ExecutionProfile::Trusted);
    let mut host = StdHost;
    eng.run_script_systems(HostPhase::Update, &mut world, &mut host).unwrap();

    let store = world.resources.get::<ScriptComponentStore>().unwrap();
    let slot = catalog.id_of("Health").unwrap();
    let layout = catalog.layout_of(slot).unwrap();
    assert_eq!(store.read_f32(slot, layout, entity, 0), Some(0.0));
}

#[test]
fn runtime_keeps_api_provider_across_load_script_package() {
    let mod_id = "runtime_combat";
    let parent = isolated_mods_parent("runtime");
    let _root = combat_mod_in(&parent, mod_id);
    let mut runtime = SparkRuntime::new();
    runtime.register_script_api_provider(Box::new(GameCombatScriptApiProvider));
    runtime.register_script_component_layout(
        "Health",
        ScriptComponentLayoutBuilder::new().field("hp", ScriptFieldKind::F32).build().unwrap(),
    );
    runtime.load_script_package(&parent).unwrap();
    let engine = runtime.script_domain().engine().expect("engine loaded");
    assert!(engine_host_has_game_combat(engine));
}

fn engine_host_has_game_combat(engine: &SparkEngine) -> bool {
    use spark_engine::engine_host_schema;
    let mut schema = engine_host_schema();
    engine.api_registry().apply_to(&mut schema);
    schema.resolve_import("game.apply_damage").is_ok() && schema.resolve_import("game.is_alive").is_ok()
}
