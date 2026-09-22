//! 按调用方键缓存 `TextureId`。
//!
//! 解码（PNG、XNB 等）留在游戏。这里只避免同一键重复上传。

use std::collections::HashMap;

use spark_types::SparkError;

use crate::{draw::DrawList, texture::TextureId};

/// 纹理句柄缓存。键由游戏决定，例如路径或图集名。
#[derive(Debug, Default, Clone)]
pub struct TextureCache {
    slots: HashMap<String, TextureId>,
}

impl TextureCache {
    /// 空缓存；与 [`Default`] 相同。
    pub fn new() -> Self {
        Self::default()
    }

    /// 按键查找已缓存的 [`TextureId`]；未命中返回 `None`。
    pub fn get(&self, key: &str) -> Option<TextureId> {
        self.slots.get(key).copied()
    }

    /// 已缓存的键数量（不等于 GPU 纹理存活数）。
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// 是否没有任何缓存键。
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// 已有键直接返回。未命中时才调用 `upload` 取像素并创建纹理。
    pub fn get_or_upload(
        &mut self,
        draw: &mut DrawList,
        key: &str,
        upload: impl FnOnce() -> Result<(u32, u32, Vec<u8>), SparkError>,
    ) -> Result<TextureId, SparkError> {
        if let Some(id) = self.get(key) {
            return Ok(id);
        }
        let (w, h, rgba) = upload()?;
        let id = draw.create_texture(w, h, rgba)?;
        self.slots.insert(key.to_string(), id);
        Ok(id)
    }

    /// 丢掉键。不会销毁 GPU 纹理，下一帧由调用方决定是否再上传。
    pub fn invalidate(&mut self, key: &str) -> Option<TextureId> {
        self.slots.remove(key)
    }
}
