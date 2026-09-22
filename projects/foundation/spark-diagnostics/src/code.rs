//! 错误命名空间与稳定错误码。

use std::{fmt, sync::Arc};

/// 错误码命名空间（如 `spark`、`spark.asset`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespaceId(Arc<str>);

impl NamespaceId {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NamespaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 命名空间内错误标识（点分，如 `not_found`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorId(Arc<str>);

impl ErrorId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 稳定错误码：`namespace` + `id` → 显示为 `spark.asset.not_found`。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorCode {
    pub namespace: NamespaceId,
    pub id: ErrorId,
}

impl ErrorCode {
    pub fn new(namespace: impl Into<Arc<str>>, id: impl Into<Arc<str>>) -> Self {
        Self { namespace: NamespaceId::new(namespace), id: ErrorId::new(id) }
    }

    /// 解析 `spark.asset.not_found`（首段为命名空间，其余为 id）。
    pub fn parse(dotted: &str) -> Self {
        if let Some((ns, rest)) = dotted.split_once('.') { Self::new(ns, rest) } else { Self::new("spark", dotted) }
    }

    pub fn as_dotted(&self) -> String {
        format!("{}.{}", self.namespace, self.id)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.namespace, self.id)
    }
}

/// 引擎保留码（最小集；领域 crate 可另定义常量）。
pub mod codes {
    use super::ErrorCode;

    pub fn not_implemented() -> ErrorCode {
        ErrorCode::new("spark", "not_implemented")
    }

    pub fn invalid_argument() -> ErrorCode {
        ErrorCode::new("spark", "invalid_argument")
    }

    pub fn invalid_state() -> ErrorCode {
        ErrorCode::new("spark", "invalid_state")
    }

    pub fn unsupported() -> ErrorCode {
        ErrorCode::new("spark", "unsupported")
    }

    pub fn resource_unavailable() -> ErrorCode {
        ErrorCode::new("spark", "resource_unavailable")
    }

    pub fn internal_invariant() -> ErrorCode {
        ErrorCode::new("spark", "internal_invariant")
    }

    pub fn io() -> ErrorCode {
        ErrorCode::new("spark", "io")
    }

    pub fn asset_not_found() -> ErrorCode {
        ErrorCode::new("spark", "asset.not_found")
    }

    pub fn asset_io() -> ErrorCode {
        ErrorCode::new("spark", "asset.io")
    }

    pub fn localization_locale_invalid() -> ErrorCode {
        ErrorCode::new("spark", "localization.locale_invalid")
    }

    pub fn image_dimension_overflow() -> ErrorCode {
        ErrorCode::new("spark", "image.dimension_overflow")
    }

    pub fn image_rgba_length_mismatch() -> ErrorCode {
        ErrorCode::new("spark", "image.rgba_length_mismatch")
    }

    pub fn image_decode() -> ErrorCode {
        ErrorCode::new("spark", "image.decode")
    }

    pub fn image_encode() -> ErrorCode {
        ErrorCode::new("spark", "image.encode")
    }

    pub fn image_pixel_out_of_bounds() -> ErrorCode {
        ErrorCode::new("spark", "image.pixel_out_of_bounds")
    }

    pub fn image_region_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.region_invalid")
    }

    pub fn image_region_out_of_bounds() -> ErrorCode {
        ErrorCode::new("spark", "image.region_out_of_bounds")
    }

    pub fn image_sprite_out_of_bounds() -> ErrorCode {
        ErrorCode::new("spark", "image.sprite_out_of_bounds")
    }

    pub fn image_sprite_grid_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.sprite_grid_invalid")
    }

    pub fn image_nine_margin_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.nine_margin_invalid")
    }

    pub fn image_dest_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.dest_invalid")
    }

    pub fn font_not_found() -> ErrorCode {
        ErrorCode::new("spark", "font.not_found")
    }

    pub fn font_parse() -> ErrorCode {
        ErrorCode::new("spark", "font.parse")
    }

    pub fn video_no_track() -> ErrorCode {
        ErrorCode::new("spark", "video.no_track")
    }

    pub fn audio_sink() -> ErrorCode {
        ErrorCode::new("spark", "audio.sink")
    }

    pub fn texture_size_invalid() -> ErrorCode {
        ErrorCode::new("spark", "texture.size_invalid")
    }

    pub fn shader_empty() -> ErrorCode {
        ErrorCode::new("spark", "shader.empty")
    }

    pub fn vfs_path_invalid() -> ErrorCode {
        ErrorCode::new("spark", "vfs.path_invalid")
    }

    pub fn gltf_invalid() -> ErrorCode {
        ErrorCode::new("spark", "gltf.invalid")
    }

    pub fn gpu_surface() -> ErrorCode {
        ErrorCode::new("spark", "gpu.surface")
    }

    pub fn gpu_adapter() -> ErrorCode {
        ErrorCode::new("spark", "gpu.adapter")
    }

    pub fn gpu_device() -> ErrorCode {
        ErrorCode::new("spark", "gpu.device")
    }

    pub fn gpu_event_loop() -> ErrorCode {
        ErrorCode::new("spark", "gpu.event_loop")
    }
}
