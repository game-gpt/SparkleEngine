//! `with_overrides`：实例补丁应用到文档副本。

use spark_prefab::{PrefabDocument, PrefabInstance};
use serde_json::json;

#[test]
fn with_overrides_patches_fields() {
    let mut doc = PrefabDocument::new("player");
    doc.ensure_component("player", "Transform")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("position".into(), json!([0, 0]));
    doc.validate().unwrap();

    let mut inst = PrefabInstance::new("assets/player.prefab", "spawn");
    inst.set_override("player/Transform.position", json!([100, 64]));

    let patched = doc.with_overrides(&inst).unwrap();
    assert_eq!(patched.nodes["player"].components["Transform"]["position"], json!([100, 64]));
    // 源文档不变
    assert_eq!(doc.nodes["player"].components["Transform"]["position"], json!([0, 0]));
}
