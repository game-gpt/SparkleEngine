//! 自 `src/cache.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_asset::{AssetCache, AssetError, BytesLoader, LoadError};
use std::io::Write;

#[test]
fn cache_load_and_reload() {
    let dir = std::env::temp_dir().join(format!("spark_asset_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.txt");
    std::fs::write(&path, b"one").unwrap();

    let loader = BytesLoader::new(&dir);
    let mut cache = AssetCache::new();
    let id = cache.load("a.txt", &loader).unwrap();
    assert_eq!(&cache.bytes(id).unwrap()[..], b"one");
    assert_eq!(cache.generation(id), Some(1));

    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(b"two").unwrap();
    drop(f);

    cache.notify_changed("a.txt", &loader).unwrap();
    assert_eq!(&cache.bytes(id).unwrap()[..], b"two");
    assert_eq!(cache.generation(id), Some(2));
    assert_eq!(cache.drain_reloads().len(), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_file() {
    let loader = BytesLoader::new(".");
    let mut cache = AssetCache::new();
    let err = cache.load("no_such_spark_asset_xyz.bin", &loader);
    assert!(matches!(err, Err(AssetError::Load(LoadError::NotFound { .. }))));
}
