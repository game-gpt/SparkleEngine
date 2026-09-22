//! [`AssetIndex`]：扫描 `.meta`，按 GUID 重命名且不换身份。

use std::fs;

use spark_asset::{AssetIndex, AssetMetaStore};
use uuid::Uuid;

#[test]
fn scan_and_rename_keeps_guid() {
    let dir = std::env::temp_dir().join(format!("spark-asset-idx-{}", Uuid::now_v7()));
    let assets = dir.join("assets");
    fs::create_dir_all(assets.join("chars")).unwrap();
    let src = assets.join("player.png");
    fs::write(&src, b"png").unwrap();
    let meta = AssetMetaStore::create(&src).unwrap();

    let mut index = AssetIndex::scan(&dir).unwrap();
    assert_eq!(index.len(), 1);
    assert_eq!(index.guid_of(&src), Some(meta.guid));
    assert_eq!(index.path_of(meta.guid), Some(src.as_path()));

    let dst = assets.join("chars").join("player.png");
    index.rename(&src, &dst).unwrap();
    assert!(!src.exists());
    assert!(dst.is_file());
    assert!(AssetMetaStore::path(&dst).is_file());
    assert_eq!(AssetMetaStore::load(&dst).unwrap().guid, meta.guid);
    assert_eq!(index.guid_of(&dst), Some(meta.guid));
    assert_eq!(index.path_of(meta.guid), Some(dst.as_path()));

    let _ = fs::remove_dir_all(&dir);
}
