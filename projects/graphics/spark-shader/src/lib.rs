//! 着色器源码与 `ShaderModule` 装载。
//!
//! WGSL 正文归本 crate；`spark-renderer-wgpu` 只消费编译结果与入口名，不内嵌着色器字符串。

#![deny(missing_docs)]
use std::sync::Arc;

use spark_types::{ErrorArg, SparkError, codes};
use wgpu::Device;

/// 引擎内建着色器（与 `src/shaders/*.wgsl` 一一对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinShader {
    /// 屏幕空间纯色四边形。
    SolidQuad,
    /// 图集采样文字 / 精灵（R 通道作 alpha）。
    TexturedGlyph,
    /// 屏幕空间 RGBA 纹理四边形（像素图集 / 图标）。
    TexturedQuad,
    /// 透视空间顶点色三角网格（无光照；天空用）。
    SolidMesh3d,
    /// 透视空间顶点色三角网格（方向光 + 雾 + ACES；不透明世界）。
    LitSolidMesh3d,
    /// 透视空间 RGBA 纹理三角网格（方向光 + 雾 + ACES）。
    TexturedMesh3d,
    /// 透视空间自发光纹理网格（无方向光；additive Emissive pass）。
    EmissiveMesh3d,
    /// 透视空间蒙皮三角网格（关节 palette + 方向光 + 雾）。
    SkinnedMesh3d,
    /// 全屏 bloom 提取 / 模糊。
    Bloom,
    /// 全屏 bloom 合成。
    BloomComposite,
    /// 视线方向大气穹顶（SkyPass；消费 `FrameLights`）。
    SkyAtmosphere3d,
    /// 深度专用（太阳阴影图；无 fragment）。
    DepthOnlyMesh3d,
}

impl BuiltinShader {
    /// wgpu / 调试用稳定标签（`spark-shader/...`），与磁盘文件名不必一一相同。
    pub const fn label(self) -> &'static str {
        match self {
            Self::SolidQuad => "spark-shader/solid-quad",
            Self::TexturedGlyph => "spark-shader/textured-glyph",
            Self::TexturedQuad => "spark-shader/textured-quad",
            Self::SolidMesh3d => "spark-shader/solid-mesh3d",
            Self::LitSolidMesh3d => "spark-shader/lit-solid-mesh3d",
            Self::TexturedMesh3d => "spark-shader/textured-mesh3d",
            Self::EmissiveMesh3d => "spark-shader/emissive-mesh3d",
            Self::SkinnedMesh3d => "spark-shader/skinned-mesh3d",
            Self::Bloom => "spark-shader/bloom",
            Self::BloomComposite => "spark-shader/bloom-composite",
            Self::SkyAtmosphere3d => "spark-shader/sky-atmosphere3d",
            Self::DepthOnlyMesh3d => "spark-shader/depth-only-mesh3d",
        }
    }

    /// 内嵌 WGSL 正文（编译期 `include_str!`）。
    pub const fn wgsl(self) -> &'static str {
        match self {
            Self::SolidQuad => include_str!("shaders/quad.wgsl"),
            Self::TexturedGlyph => include_str!("shaders/text.wgsl"),
            Self::TexturedQuad => include_str!("shaders/tex_quad.wgsl"),
            Self::SolidMesh3d => include_str!("shaders/mesh3d.wgsl"),
            Self::LitSolidMesh3d => include_str!("shaders/mesh3d_lit.wgsl"),
            Self::TexturedMesh3d => include_str!("shaders/mesh3d_tex.wgsl"),
            Self::EmissiveMesh3d => include_str!("shaders/mesh3d_tex_emissive.wgsl"),
            Self::SkinnedMesh3d => include_str!("shaders/mesh3d_skinned.wgsl"),
            Self::Bloom => include_str!("shaders/bloom.wgsl"),
            Self::BloomComposite => include_str!("shaders/bloom_composite.wgsl"),
            Self::SkyAtmosphere3d => include_str!("shaders/sky_atmosphere.wgsl"),
            Self::DepthOnlyMesh3d => include_str!("shaders/mesh3d_depth.wgsl"),
        }
    }

    /// 顶点着色器入口名。内建着色器统一为 `vs_main`。
    pub const fn vertex_entry(self) -> &'static str {
        "vs_main"
    }

    /// 片元着色器入口名。内建着色器统一为 `fs_main`（深度-only 管线可忽略）。
    pub const fn fragment_entry(self) -> &'static str {
        "fs_main"
    }
}

/// 任意 WGSL 源（内建或游戏侧注入）。
#[derive(Debug, Clone, Copy)]
pub struct ShaderSource<'a> {
    /// 调试 / `ShaderModuleDescriptor` 标签。
    pub label: &'a str,
    /// WGSL 源码正文。
    pub wgsl: &'a str,
}

impl<'a> From<BuiltinShader> for ShaderSource<'a> {
    fn from(value: BuiltinShader) -> Self {
        Self { label: value.label(), wgsl: value.wgsl() }
    }
}

/// 在设备上创建 `ShaderModule`。
pub fn create_module(device: &Device, source: ShaderSource<'_>) -> wgpu::ShaderModule {
    device
        .create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(source.label), source: wgpu::ShaderSource::Wgsl(source.wgsl.into()) })
}

/// 创建内建着色器模块。
pub fn create_builtin(device: &Device, builtin: BuiltinShader) -> wgpu::ShaderModule {
    create_module(device, builtin.into())
}

/// 校验 WGSL 非空（装载前快速失败；真正编译错误仍由 wgpu 报告）。
pub fn validate_source(source: &ShaderSource<'_>) -> Result<(), SparkError> {
    if source.wgsl.trim().is_empty() {
        return Err(SparkError::new(codes::shader_empty()).arg("label", ErrorArg::String(Arc::from(source.label))));
    }
    Ok(())
}
