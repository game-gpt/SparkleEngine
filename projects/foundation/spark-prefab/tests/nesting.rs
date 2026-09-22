//! 嵌套 Prefab 环检测。

use std::fs;

use spark_prefab::{PrefabDocument, PrefabError, nested_ref, validate_prefab_file};
use uuid::Uuid;

#[test]
fn nesting_cycle_detected() {
    let dir = std::env::temp_dir().join(format!("spark-prefab-cycle-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let a_path = dir.join("a.prefab");
    let b_path = dir.join("b.prefab");

    let mut a = PrefabDocument::new("root_a");
    a.ensure_node("nest").prefab = Some(nested_ref(b_path.to_string_lossy()));
    a.ensure_child("root_a", "nest").unwrap();
    a.save(&a_path).unwrap();

    let mut b = PrefabDocument::new("root_b");
    b.ensure_node("nest").prefab = Some(nested_ref(a_path.to_string_lossy()));
    b.ensure_child("root_b", "nest").unwrap();
    b.save(&b_path).unwrap();

    let err = validate_prefab_file(&a_path).unwrap_err();
    assert!(matches!(err, PrefabError::PrefabCycle { .. }));
    assert_eq!(err.code(), "spark.prefab.prefab_cycle");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn nesting_ok_without_cycle() {
    let dir = std::env::temp_dir().join(format!("spark-prefab-nest-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let sword = dir.join("sword.prefab");
    let player = dir.join("player.prefab");

    PrefabDocument::new("sword").save(&sword).unwrap();

    let mut p = PrefabDocument::new("player");
    p.ensure_node("weapon").prefab = Some(nested_ref(sword.to_string_lossy()));
    p.ensure_child("player", "weapon").unwrap();
    p.save(&player).unwrap();

    let loaded = validate_prefab_file(&player).unwrap();
    assert_eq!(loaded.root, "player");

    let _ = fs::remove_dir_all(&dir);
}
