//! Prefab 文档读写与幂等 ensure。

use spark_prefab::{PREFAB_SCHEMA, PrefabDocument, PrefabError};
use serde_json::json;

#[test]
fn new_roundtrip_pretty_json() {
    let mut doc = PrefabDocument::new("player");
    doc.ensure_child("player", "sprite").unwrap();
    doc.ensure_component("player", "Transform").unwrap();
    doc.set_component(
        "sprite",
        "Sprite",
        json!({ "texture": "assets/player.png" }),
    )
    .unwrap();
    doc.validate().unwrap();

    let text = doc.to_string_pretty().unwrap();
    assert!(text.contains(PREFAB_SCHEMA));
    let back = PrefabDocument::from_str(&text).unwrap();
    assert_eq!(back.root, "player");
    assert!(back.nodes["player"].children.contains(&"sprite".into()));
    assert_eq!(back.nodes["sprite"].components["Sprite"]["texture"], "assets/player.png");
}

#[test]
fn ensure_is_idempotent() {
    let mut doc = PrefabDocument::new("player");
    doc.ensure_child("player", "body").unwrap();
    doc.ensure_child("player", "body").unwrap();
    assert_eq!(doc.nodes["player"].children, vec!["body".to_string()]);
}

#[test]
fn bad_node_id_rejected() {
    let mut doc = PrefabDocument::new("player");
    doc.nodes.insert(
        "a/b".into(),
        spark_prefab::PrefabNode {
            name: None,
            components: Default::default(),
            children: vec![],
            prefab: None,
        },
    );
    let err = doc.validate().unwrap_err();
    assert_eq!(err.code(), "spark.prefab.bad_node_id");
}

#[test]
fn missing_child_edge() {
    let mut doc = PrefabDocument::new("player");
    doc.nodes.get_mut("player").unwrap().children.push("ghost".into());
    assert!(matches!(doc.validate(), Err(PrefabError::ChildMissing { .. })));
}
