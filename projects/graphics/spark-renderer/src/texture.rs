//! 纹理句柄分配。
//!
//! 权威上传对象为 [`spark_texture::TextureUpload`]（由本 crate re-export）。

use std::sync::atomic::{AtomicU32, Ordering};

/// 不透明纹理 ID。进程内单调分配，跨帧可缓存。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);

static NEXT_TEXTURE_ID: AtomicU32 = AtomicU32::new(1);

/// 分配稳定 [`TextureId`]（勿每帧重复分配同一逻辑贴图）。
pub fn alloc_texture_id() -> TextureId {
    TextureId(NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed))
}
