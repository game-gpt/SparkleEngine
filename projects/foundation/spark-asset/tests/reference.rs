//! [`AssetRef`]：路径为主，GUID 由工具附着。

use std::fs;

use spark_asset::{AssetMetaError, AssetMetaStore, AssetRef};
use uuid::Uuid;

#[test]
fn path_ref_serializes_as_string() {
    let r = AssetRef::from_path("assets/player.png");
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, "\"assets/player.png\"");
    let back: AssetRef = serde_json::from_str(&json).unwrap();
    assert_eq!(back.path(), "assets/player.png");
    assert!(back.guid().is_none());
}

#[test]
fn resolve_attaches_guid_from_meta() {
    let dir = std::env::temp_dir().join(format!("spark-asset-ref-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let asset = dir.join("player.png");
    fs::write(&asset, b"png").unwrap();
    let created = AssetMetaStore::create(&asset).unwrap();

    let resolved = AssetRef::from_path(asset.to_string_lossy()).resolve().unwrap();
    assert_eq!(resolved.guid(), Some(created.guid));
    assert_eq!(resolved.path(), asset.to_string_lossy());

    let obj = serde_json::to_value(&resolved).unwrap();
    assert_eq!(obj["path"], asset.to_string_lossy().as_ref());
    assert_eq!(obj["guid"], created.guid.to_string());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_detects_guid_mismatch() {
    let dir = std::env::temp_dir().join(format!("spark-asset-ref-mm-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let asset = dir.join("tex.png");
    fs::write(&asset, b"x").unwrap();
    let meta = AssetMetaStore::create(&asset).unwrap();

    let wrong = AssetRef::Resolved {
        path: asset.to_string_lossy().into_owned(),
        guid: Uuid::nil(),
    };
    let err = wrong.validate().unwrap_err();
    assert!(matches!(err, AssetMetaError::GuidMismatch { .. }));
    assert_eq!(err.code(), "spark.asset.guid_mismatch");

    let ok = AssetRef::Resolved {
        path: asset.to_string_lossy().into_owned(),
        guid: meta.guid,
    };
    assert_eq!(ok.validate().unwrap().guid, meta.guid);

    let _ = fs::remove_dir_all(&dir);
}
