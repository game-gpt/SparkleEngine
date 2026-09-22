//! 错误命名空间与稳定错误码。

use std::{fmt, sync::Arc};

/// 错误码命名空间（如 `spark`、`spark.asset`）。
///
/// 命名空间与 id 组合后形成稳定点分码；变更已发布字符串视为破坏性变更。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespaceId(Arc<str>);

impl NamespaceId {
    /// 由字符串构造；调用方保证非空且为稳定英文标识。
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    /// 命名空间原文。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NamespaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 命名空间内错误标识（点分，如 `not_found` 或 `asset.not_found`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorId(Arc<str>);

impl ErrorId {
    /// 由字符串构造；可含点号表示子域。
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// 标识原文。
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
///
/// 权威身份仅此二者；自然语言句子不在此结构内。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorCode {
    /// 命名空间段。
    pub namespace: NamespaceId,
    /// 命名空间内标识（可再含点号）。
    pub id: ErrorId,
}

impl ErrorCode {
    /// 组合命名空间与 id。
    pub fn new(namespace: impl Into<Arc<str>>, id: impl Into<Arc<str>>) -> Self {
        Self { namespace: NamespaceId::new(namespace), id: ErrorId::new(id) }
    }

    /// 解析 `spark.asset.not_found`（首段为命名空间，其余为 id）。
    ///
    /// 无点号时命名空间默认为 `spark`，整串作为 id。
    pub fn parse(dotted: &str) -> Self {
        if let Some((ns, rest)) = dotted.split_once('.') { Self::new(ns, rest) } else { Self::new("spark", dotted) }
    }

    /// 格式化为点分字符串（与 [`Display`] 一致）。
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
///
/// 每个函数返回固定点分码；调用方用 [`crate::Error::new`] 挂参数。
pub mod codes {
    use super::ErrorCode;

    /// 能力尚未实现（参数常见 `what`）。
    pub fn not_implemented() -> ErrorCode {
        ErrorCode::new("spark", "not_implemented")
    }

    /// 调用参数非法或不在允许域（参数常见 `name`）。
    pub fn invalid_argument() -> ErrorCode {
        ErrorCode::new("spark", "invalid_argument")
    }

    /// 对象处于不允许该操作的状态（参数常见 `detail`）。
    pub fn invalid_state() -> ErrorCode {
        ErrorCode::new("spark", "invalid_state")
    }

    /// 当前平台 / 配置不支持该功能。
    pub fn unsupported() -> ErrorCode {
        ErrorCode::new("spark", "unsupported")
    }

    /// 依赖资源暂时不可用（设备、句柄、后端服务等）。
    pub fn resource_unavailable() -> ErrorCode {
        ErrorCode::new("spark", "resource_unavailable")
    }

    /// 引擎内部不变式被破坏（应视为缺陷）。
    pub fn internal_invariant() -> ErrorCode {
        ErrorCode::new("spark", "internal_invariant")
    }

    /// 通用 I/O 失败（非资产专用路径请优先用更具体码）。
    pub fn io() -> ErrorCode {
        ErrorCode::new("spark", "io")
    }

    /// 逻辑资源键在目录中不存在。
    pub fn asset_not_found() -> ErrorCode {
        ErrorCode::new("spark", "asset.not_found")
    }

    /// 资产读写 I/O 失败。
    pub fn asset_io() -> ErrorCode {
        ErrorCode::new("spark", "asset.io")
    }

    /// 区域 / 语言标签无法解析或不受支持。
    pub fn localization_locale_invalid() -> ErrorCode {
        ErrorCode::new("spark", "localization.locale_invalid")
    }

    /// 图像宽高乘积溢出可表示范围。
    pub fn image_dimension_overflow() -> ErrorCode {
        ErrorCode::new("spark", "image.dimension_overflow")
    }

    /// RGBA 缓冲字节数与宽×高×4 不一致。
    pub fn image_rgba_length_mismatch() -> ErrorCode {
        ErrorCode::new("spark", "image.rgba_length_mismatch")
    }

    /// 图像解码失败（格式损坏或不支持）。
    pub fn image_decode() -> ErrorCode {
        ErrorCode::new("spark", "image.decode")
    }

    /// 图像编码失败。
    pub fn image_encode() -> ErrorCode {
        ErrorCode::new("spark", "image.encode")
    }

    /// 像素坐标越出图像边界。
    pub fn image_pixel_out_of_bounds() -> ErrorCode {
        ErrorCode::new("spark", "image.pixel_out_of_bounds")
    }

    /// 区域矩形自身非法（宽高非正等）。
    pub fn image_region_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.region_invalid")
    }

    /// 区域矩形越出图像边界。
    pub fn image_region_out_of_bounds() -> ErrorCode {
        ErrorCode::new("spark", "image.region_out_of_bounds")
    }

    /// 精灵帧矩形越出图集边界。
    pub fn image_sprite_out_of_bounds() -> ErrorCode {
        ErrorCode::new("spark", "image.sprite_out_of_bounds")
    }

    /// 精灵网格划分参数非法（行列或单元格尺寸）。
    pub fn image_sprite_grid_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.sprite_grid_invalid")
    }

    /// 九宫格边距之和超过对应边长。
    pub fn image_nine_margin_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.nine_margin_invalid")
    }

    ///  blit / 拷贝目标矩形非法。
    pub fn image_dest_invalid() -> ErrorCode {
        ErrorCode::new("spark", "image.dest_invalid")
    }

    /// 字体资源未注册或路径无效。
    pub fn font_not_found() -> ErrorCode {
        ErrorCode::new("spark", "font.not_found")
    }

    /// 字体文件解析失败。
    pub fn font_parse() -> ErrorCode {
        ErrorCode::new("spark", "font.parse")
    }

    /// 视频容器中找不到可用轨。
    pub fn video_no_track() -> ErrorCode {
        ErrorCode::new("spark", "video.no_track")
    }

    /// 音频输出端（sink）创建或写入失败。
    pub fn audio_sink() -> ErrorCode {
        ErrorCode::new("spark", "audio.sink")
    }

    /// 纹理宽高为零或超出设备上限。
    pub fn texture_size_invalid() -> ErrorCode {
        ErrorCode::new("spark", "texture.size_invalid")
    }

    /// 上传缓冲长度与纹理布局推算字节数不符。
    pub fn texture_data_length_mismatch() -> ErrorCode {
        ErrorCode::new("spark", "texture.data_length_mismatch")
    }

    /// 纹理布局（mip / 数组层 / 块对齐）非法。
    pub fn texture_layout_invalid() -> ErrorCode {
        ErrorCode::new("spark", "texture.layout_invalid")
    }

    /// 像素格式当前后端不支持。
    pub fn texture_format_unsupported() -> ErrorCode {
        ErrorCode::new("spark", "texture.format_unsupported")
    }

    /// 纹理上传区域或用法非法。
    pub fn texture_upload_invalid() -> ErrorCode {
        ErrorCode::new("spark", "texture.upload_invalid")
    }

    /// 着色器源码为空。
    pub fn shader_empty() -> ErrorCode {
        ErrorCode::new("spark", "shader.empty")
    }

    /// 虚拟文件系统路径语法或根约束非法。
    pub fn vfs_path_invalid() -> ErrorCode {
        ErrorCode::new("spark", "vfs.path_invalid")
    }

    /// glTF 文档结构或引用非法。
    pub fn gltf_invalid() -> ErrorCode {
        ErrorCode::new("spark", "gltf.invalid")
    }

    /// GPU 表面（swapchain / 窗口表面）获取或配置失败。
    pub fn gpu_surface() -> ErrorCode {
        ErrorCode::new("spark", "gpu.surface")
    }

    /// 无可用 GPU 适配器。
    pub fn gpu_adapter() -> ErrorCode {
        ErrorCode::new("spark", "gpu.adapter")
    }

    /// 逻辑设备创建或特性请求失败。
    pub fn gpu_device() -> ErrorCode {
        ErrorCode::new("spark", "gpu.device")
    }

    /// 窗口 / 平台事件循环启动或泵送失败。
    pub fn gpu_event_loop() -> ErrorCode {
        ErrorCode::new("spark", "gpu.event_loop")
    }
}
