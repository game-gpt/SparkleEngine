//! `save_registered`：写 Prefab 并登记 `.meta`。

use std::fs;

use spark_asset::AssetMetaStore;
use spark_prefab::{PrefabDocument, save_registered};
use uuid::Uuid;

#[test]
fn save_registered_creates_meta_with_prefab_kind() {
    let dir = std::env::temp_dir().join(format!("spark-prefab-reg-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("player.prefab");

    let doc = PrefabDocument::new("player");
    let meta = save_registered(&doc, &path).unwrap();
    assert_eq!(meta.kind.as_deref(), Some("prefab"));
    assert!(path.is_file());
    assert!(AssetMetaStore::path(&path).is_file());

    let again = save_registered(&doc, &path).unwrap();
    assert_eq!(again.guid, meta.guid);

    let _ = fs::remove_dir_all(&dir);
}
