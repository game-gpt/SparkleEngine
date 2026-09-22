//! 自 `src/hot_reload.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_asset::{AssetCache, BytesLoader, HotReloadWatch};
use std::{io::Write, thread, time::Duration};

#[test]
fn hot_reload_watch_polls_mtime() {
    let dir = std::env::temp_dir().join(format!("spark_asset_watch_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("b.txt");
    std::fs::write(&path, b"one").unwrap();

    let loader = BytesLoader::new(&dir);
    let mut cache = AssetCache::new();
    let id = cache.load("b.txt", &loader).unwrap();

    let mut watch = HotReloadWatch::new();
    watch.watch_key("b.txt", &loader);
    assert!(watch.poll(&mut cache, &loader).unwrap().is_empty());

    thread::sleep(Duration::from_millis(30));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(b"two").unwrap();
    drop(f);

    let evs = watch.poll(&mut cache, &loader).unwrap();
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].key.as_str(), "b.txt");
    assert_eq!(&cache.bytes(id).unwrap()[..], b"two");

    let _ = std::fs::remove_dir_all(&dir);
}
