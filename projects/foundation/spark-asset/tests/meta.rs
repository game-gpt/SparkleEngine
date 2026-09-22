//! 旁车 `.meta`：身份生成、持久化、缺失不静默换 GUID。

use std::fs;

use spark_asset::{AssetMetaError, AssetMetaStore};
use uuid::Uuid;

#[test]
fn meta_path_appends_suffix() {
    let p = AssetMetaStore::path("assets/image.png");
    assert_eq!(p, std::path::Path::new("assets/image.png.meta"));
}

#[test]
fn create_writes_uuid_v7_and_reload_keeps_guid() {
    let dir = std::env::temp_dir().join(format!("spark-asset-meta-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let asset = dir.join("image.png");
    fs::write(&asset, b"png").unwrap();

    let created = AssetMetaStore::create(&asset).unwrap();
    assert_eq!(created.format, 1);
    assert_eq!(created.guid.get_version(), Some(uuid::Version::SortRand));

    let meta_path = AssetMetaStore::path(&asset);
    assert!(meta_path.is_file());

    let loaded = AssetMetaStore::load(&asset).unwrap();
    assert_eq!(loaded.guid, created.guid);

    let again = AssetMetaStore::load_or_create(&asset).unwrap();
    assert_eq!(again.guid, created.guid);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_missing_does_not_create() {
    let dir = std::env::temp_dir().join(format!("spark-asset-meta-miss-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let asset = dir.join("orphan.png");
    fs::write(&asset, b"x").unwrap();

    let err = AssetMetaStore::load(&asset).unwrap_err();
    assert!(matches!(err, AssetMetaError::Missing { .. }));
    assert_eq!(err.code(), "spark.asset.meta_missing");
    assert!(!AssetMetaStore::path(&asset).exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_refuses_overwrite() {
    let dir = std::env::temp_dir().join(format!("spark-asset-meta-exists-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let asset = dir.join("tex.png");
    fs::write(&asset, b"x").unwrap();

    let first = AssetMetaStore::create(&asset).unwrap();
    let err = AssetMetaStore::create(&asset).unwrap_err();
    assert!(matches!(err, AssetMetaError::AlreadyExists { .. }));
    assert_eq!(AssetMetaStore::load(&asset).unwrap().guid, first.guid);

    let _ = fs::remove_dir_all(&dir);
}
