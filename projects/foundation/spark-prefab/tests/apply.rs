//! `with_overrides`：实例补丁应用到文档副本。

use std::collections::BTreeMap;

use spark_asset::MetaValue;
use spark_prefab::{PrefabDocument, PrefabInstance};

#[test]
fn with_overrides_patches_fields() {
    let mut doc = PrefabDocument::new("player");
    doc.set_component(
        "player",
        "Transform",
        MetaValue::Table(BTreeMap::from([("position".into(), MetaValue::Array(vec![MetaValue::Int(0), MetaValue::Int(0)]))])),
    )
    .unwrap();
    doc.validate().unwrap();

    let mut inst = PrefabInstance::new("assets/player.prefab", "spawn");
    inst.set_override("player/Transform.position", MetaValue::Array(vec![MetaValue::Int(100), MetaValue::Int(64)]));

    let patched = doc.with_overrides(&inst).unwrap();
    match &patched.nodes["player"].components["Transform"] {
        MetaValue::Table(t) => {
            assert_eq!(t["position"], MetaValue::Array(vec![MetaValue::Int(100), MetaValue::Int(64)]));
        }
        other => panic!("{other:?}"),
    }
    match &doc.nodes["player"].components["Transform"] {
        MetaValue::Table(t) => {
            assert_eq!(t["position"], MetaValue::Array(vec![MetaValue::Int(0), MetaValue::Int(0)]));
        }
        other => panic!("{other:?}"),
    }
}
