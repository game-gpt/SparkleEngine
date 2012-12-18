//! 基于 mtime 轮询的热重载监视。

use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use crate::{
    AssetError,
    cache::AssetCache,
    handle::AssetKey,
    loader::{AssetLoader, BytesLoader, ReloadEvent},
};

#[derive(Debug, Clone)]
struct WatchEntry {
    path: PathBuf,
    mtime: Option<SystemTime>,
}

/// 轮询已登记资源的修改时间，变化时驱动 [`AssetCache::notify_changed`]。
#[derive(Debug, Default)]
pub struct HotReloadWatch {
    watched: HashMap<AssetKey, WatchEntry>,
}

impl HotReloadWatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// 监视 [`BytesLoader`] 解析出的路径。
    pub fn watch_key(&mut self, key: impl Into<AssetKey>, loader: &BytesLoader) {
        let key = key.into();
        let path = loader.resolve(&key);
        self.watch_path(key, path);
    }

    pub fn watch_path(&mut self, key: impl Into<AssetKey>, path: impl Into<PathBuf>) {
        let key = key.into();
        let path = path.into();
        let mtime = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        self.watched.insert(key, WatchEntry { path, mtime });
    }

    pub fn unwatch(&mut self, key: &AssetKey) -> bool {
        self.watched.remove(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.watched.len()
    }

    pub fn is_empty(&self) -> bool {
        self.watched.is_empty()
    }

    /// 扫描 mtime；有变化则重载缓存。返回本轮事件（带真实路径）。
    pub fn poll(&mut self, cache: &mut AssetCache, loader: &dyn AssetLoader) -> Result<Vec<ReloadEvent>, AssetError> {
        let mut changed = Vec::new();
        for (key, entry) in self.watched.iter_mut() {
            let new_mtime = std::fs::metadata(&entry.path).ok().and_then(|m| m.modified().ok());
            let did_change = match (entry.mtime, new_mtime) {
                (Some(old), Some(new)) => new > old,
                (None, Some(_)) => true,
                _ => false,
            };
            if did_change {
                entry.mtime = new_mtime;
                changed.push((key.clone(), entry.path.clone()));
            }
        }

        let before = cache.reload_count();
        for (key, _) in &changed {
            cache.notify_changed(key.clone(), loader)?;
        }
        let mut events = cache.drain_reloads();
        // 丢掉 poll 之前残留，只保留本轮；并覆写真实路径
        if events.len() > changed.len() {
            events = events.split_off(events.len() - changed.len());
        }
        let _ = before;
        for (ev, (_, path)) in events.iter_mut().zip(changed.iter()) {
            ev.path = path.clone();
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetCache, loader::BytesLoader};
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
}
