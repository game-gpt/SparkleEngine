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
///
/// 单位：依赖文件系统 `modified()` 时间戳；分辨率随平台变化。
#[derive(Debug, Default)]
pub struct HotReloadWatch {
    watched: HashMap<AssetKey, WatchEntry>,
}

impl HotReloadWatch {
    /// 空监视表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 监视 [`BytesLoader`] 解析出的路径。
    pub fn watch_key(&mut self, key: impl Into<AssetKey>, loader: &BytesLoader) {
        let key = key.into();
        let path = loader.resolve(&key);
        self.watch_path(key, path);
    }

    /// 登记 `(key, path)`；以当前 mtime（若可得）为基线，下次 `poll` 才报变更。
    pub fn watch_path(&mut self, key: impl Into<AssetKey>, path: impl Into<PathBuf>) {
        let key = key.into();
        let path = path.into();
        let mtime = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        self.watched.insert(key, WatchEntry { path, mtime });
    }

    /// 取消监视；曾登记返回 `true`。
    pub fn unwatch(&mut self, key: &AssetKey) -> bool {
        self.watched.remove(key).is_some()
    }

    /// 监视条目数。
    pub fn len(&self) -> usize {
        self.watched.len()
    }

    /// 是否无监视项。
    pub fn is_empty(&self) -> bool {
        self.watched.is_empty()
    }

    /// 扫描 mtime；有变化则重载缓存。返回本轮事件（带真实路径）。
    ///
    /// 任一条 `notify_changed` 失败则整轮返回错误；已成功写入缓存的变更不会回滚。
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
