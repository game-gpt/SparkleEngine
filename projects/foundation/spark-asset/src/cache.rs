//! 内存缓存与热重载队列。

use std::{collections::HashMap, sync::Arc};

use crate::{
    AssetError,
    handle::{AssetId, AssetKey},
    loader::{AssetLoader, ReloadEvent},
};

#[derive(Debug, Clone)]
struct Entry {
    key: AssetKey,
    bytes: Arc<[u8]>,
    generation: u32,
}

#[derive(Debug)]
pub struct AssetCache {
    by_key: HashMap<AssetKey, AssetId>,
    entries: Vec<Option<Entry>>,
    free: Vec<u32>,
    reloads: Vec<ReloadEvent>,
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetCache {
    pub fn new() -> Self {
        Self { by_key: HashMap::new(), entries: Vec::new(), free: Vec::new(), reloads: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_id(&self, key: &AssetKey) -> Option<AssetId> {
        self.by_key.get(key).copied()
    }

    pub fn bytes(&self, id: AssetId) -> Option<Arc<[u8]>> {
        self.entries.get(id.0 as usize).and_then(|e| e.as_ref()).map(|e| Arc::clone(&e.bytes))
    }

    pub fn generation(&self, id: AssetId) -> Option<u32> {
        self.entries.get(id.0 as usize).and_then(|e| e.as_ref()).map(|e| e.generation)
    }

    /// 加载或返回已缓存句柄。
    pub fn load(&mut self, key: impl Into<AssetKey>, loader: &dyn AssetLoader) -> Result<AssetId, AssetError> {
        let key = key.into();
        if let Some(id) = self.by_key.get(&key).copied() {
            return Ok(id);
        }
        let data = loader.load(&key)?;
        Ok(self.insert(key, data))
    }

    pub fn insert(&mut self, key: AssetKey, data: impl Into<Vec<u8>>) -> AssetId {
        let bytes: Arc<[u8]> = Arc::from(data.into().into_boxed_slice());
        let entry = Entry { key: key.clone(), bytes, generation: 1 };
        let id = if let Some(slot) = self.free.pop() {
            self.entries[slot as usize] = Some(entry);
            AssetId(slot)
        }
        else {
            let id = self.entries.len() as u32;
            self.entries.push(Some(entry));
            AssetId(id)
        };
        self.by_key.insert(key, id);
        id
    }

    /// 宿主检测到文件变化时调用：重新加载并推送 [`ReloadEvent`]。
    pub fn notify_changed(&mut self, key: impl Into<AssetKey>, loader: &dyn AssetLoader) -> Result<AssetId, AssetError> {
        let key = key.into();
        let data = loader.load(&key).map_err(AssetError::from)?;
        if let Some(id) = self.by_key.get(&key).copied() {
            if let Some(Some(entry)) = self.entries.get_mut(id.0 as usize) {
                entry.bytes = Arc::from(data.into_boxed_slice());
                entry.generation = entry.generation.wrapping_add(1);
                self.reloads.push(ReloadEvent::new(key.clone(), key.as_str()));
                return Ok(id);
            }
        }
        let id = self.insert(key.clone(), data);
        self.reloads.push(ReloadEvent::new(key.clone(), key.as_str()));
        Ok(id)
    }

    pub fn drain_reloads(&mut self) -> Vec<ReloadEvent> {
        std::mem::take(&mut self.reloads)
    }

    pub fn reload_count(&self) -> usize {
        self.reloads.len()
    }

    pub fn remove(&mut self, key: &AssetKey) -> bool {
        let Some(id) = self.by_key.remove(key)
        else {
            return false;
        };
        if let Some(slot) = self.entries.get_mut(id.0 as usize) {
            *slot = None;
            self.free.push(id.0);
            true
        }
        else {
            false
        }
    }
}
