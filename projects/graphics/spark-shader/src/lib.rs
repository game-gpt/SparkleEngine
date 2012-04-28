//! 着色器源码与 `ShaderModule` 装载。
//!
//! WGSL 正文归本 crate；`spark-renderer-wgpu` 只消费编译结果与入口名，不内嵌着色器字符串。

use spark_core::SparkError;
use wgpu::Device;

/// 引擎内建着色器（与 `src/shaders/*.wgsl` 一一对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinShader {
    /// 屏幕空间纯色四边形。
    SolidQuad,
    /// 图集采样文字 / 精灵（R 通道作 alpha）。
    TexturedGlyph,
    /// 透视空间顶点色三角网格。
    SolidMesh3d,
    /// 透视空间 RGBA 纹理三角网格。
    TexturedMesh3d,
}

impl BuiltinShader {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SolidQuad => "spark-shader/solid-quad",
            Self::TexturedGlyph => "spark-shader/textured-glyph",
            Self::SolidMesh3d => "spark-shader/solid-mesh3d",
            Self::TexturedMesh3d => "spark-shader/textured-mesh3d",
        }
    }

    pub const fn wgsl(self) -> &'static str {
        match self {
            Self::SolidQuad => include_str!("shaders/quad.wgsl"),
            Self::TexturedGlyph => include_str!("shaders/text.wgsl"),
            Self::SolidMesh3d => include_str!("shaders/mesh3d.wgsl"),
            Self::TexturedMesh3d => include_str!("shaders/mesh3d_tex.wgsl"),
        }
    }

    pub const fn vertex_entry(self) -> &'static str {
        "vs_main"
    }

    pub const fn fragment_entry(self) -> &'static str {
        "fs_main"
    }
}

/// 任意 WGSL 源（内建或游戏侧注入）。
#[derive(Debug, Clone, Copy)]
pub struct ShaderSource<'a> {
    pub label: &'a str,
    pub wgsl: &'a str,
}

impl<'a> From<BuiltinShader> for ShaderSource<'a> {
    fn from(value: BuiltinShader) -> Self {
        Self {
            label: value.label(),
            wgsl: value.wgsl(),
        }
    }
}

/// 在设备上创建 `ShaderModule`。
pub fn create_module(device: &Device, source: ShaderSource<'_>) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(source.label),
        source: wgpu::ShaderSource::Wgsl(source.wgsl.into()),
    })
}

/// 创建内建着色器模块。
pub fn create_builtin(device: &Device, builtin: BuiltinShader) -> wgpu::ShaderModule {
    create_module(device, builtin.into())
}

/// 校验 WGSL 非空（装载前快速失败；真正编译错误仍由 wgpu 报告）。
pub fn validate_source(source: &ShaderSource<'_>) -> Result<(), SparkError> {
    if source.wgsl.trim().is_empty() {
        return Err(SparkError::Message(format!(
            "着色器源码为空：{}",
            source.label
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_sources_are_nonempty() {
        for s in [
            BuiltinShader::SolidQuad,
            BuiltinShader::TexturedGlyph,
            BuiltinShader::SolidMesh3d,
            BuiltinShader::TexturedMesh3d,
        ] {
            validate_source(&s.into()).unwrap();
            assert!(s.wgsl().contains("vs_main"));
            assert!(s.wgsl().contains("fs_main"));
        }
    }
}
