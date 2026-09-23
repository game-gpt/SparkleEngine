//! 绑定期 QueryPlan 批量查询原生（`query_batch_*`）。

use spark_ecs::World;
use spark_engine::{SCRIPT_MARKER_NAME, ScriptSystemDescriptor, SparkEngine};
use spark_script::HostPhase;
use spark_vm::StdHost;

fn write_batch_mod(mod_id: &str, main_vk: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("spark_engine_{mod_id}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        format!(
            "id = \"{mod_id}\"\nversion = \"0.0.0\"\nentry = \"main.vk\"\n"
        ),
    )
    .unwrap();
    std::fs::write(root.join("main.vk"), main_vk).unwrap();
    root
}

fn setup_engine(mod_id: &str, main_vk: &str) -> (SparkEngine, ScriptSystemDescriptor, World, StdHost) {
    let root = write_batch_mod(mod_id, main_vk);
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    let desc = ScriptSystemDescriptor::new(mod_id, "update", "update", HostPhase::Update)
        .read(SCRIPT_MARKER_NAME)
        .query_archetype("rock");
    eng.register_script_system(desc.clone()).unwrap();
    let mut world = World::new();
    eng.apply_script_commands_to_world(&mut world).unwrap();
    (eng, desc, world, StdHost)
}

fn run_update(
    eng: &mut SparkEngine,
    mod_id: &str,
    desc: &ScriptSystemDescriptor,
    world: &World,
    host: &mut StdHost,
) -> spark_gc::Value {
    let plan = eng.query_plan(desc);
    eng.refresh_script_query(world);
    let catalog = eng.component_catalog().clone();
    eng.shared().borrow_mut().begin_script_call(HostPhase::Update, Some(desc), plan.as_ref(), &catalog, None);
    let v = eng
        .get_mod_mut(mod_id)
        .unwrap()
        .domain
        .as_mut()
        .unwrap()
        .call_in_phase("update", &[], HostPhase::Update, host)
        .unwrap();
    eng.shared().borrow_mut().end_script_call();
    v
}

#[test]
fn query_batch_count_reads_column_batch() {
    let (mut eng, desc, world, mut host) = setup_engine(
        "batch_query_count",
        r#"
        micro on_load() {
            queue_spawn("rock")
            queue_spawn("rock")
            return 0
        }
        micro update() {
            return query_batch_count(0)
        }
        return 0
        "#,
    );
    let v = run_update(&mut eng, "batch_query_count", &desc, &world, &mut host);
    assert_eq!(v.as_number(), Some(2.0));
}

#[test]
fn column_batch_entity_bits_match_snapshot() {
    let (mut eng, desc, world, _host) = setup_engine(
        "batch_query_entity",
        r#"
        micro on_load() {
            queue_spawn("rock")
            return 0
        }
        micro update() {
            return 0
        }
        return 0
        "#,
    );
    let plan = eng.query_plan(&desc).unwrap();
    eng.refresh_script_query(&world);
    let catalog = eng.component_catalog().clone();
    eng.shared().borrow_mut().begin_script_call(HostPhase::Update, Some(&desc), Some(&plan), &catalog, None);
    let expected = eng.shared().borrow().query.entity_at("rock", 0).unwrap();
    let batch = eng.shared().borrow().active_column_batch.clone().unwrap();
    eng.shared().borrow_mut().end_script_call();
    assert!(batch.entity_count(0) >= 1);
    assert_eq!(batch.views()[0].entity_bits(0), Some(expected));
}
