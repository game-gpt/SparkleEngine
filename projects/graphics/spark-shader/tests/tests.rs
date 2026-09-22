//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_shader::*;

#[test]
fn builtin_sources_are_nonempty() {
    for s in [
        BuiltinShader::SolidQuad,
        BuiltinShader::TexturedGlyph,
        BuiltinShader::TexturedQuad,
        BuiltinShader::SolidMesh3d,
        BuiltinShader::LitSolidMesh3d,
        BuiltinShader::TexturedMesh3d,
        BuiltinShader::EmissiveMesh3d,
        BuiltinShader::SkinnedMesh3d,
        BuiltinShader::Bloom,
        BuiltinShader::BloomComposite,
        BuiltinShader::SkyAtmosphere3d,
        BuiltinShader::DepthOnlyMesh3d,
    ] {
        validate_source(&s.into()).unwrap();
        assert!(s.wgsl().contains("vs_main"));
        assert!(
            s.wgsl().contains("fs_main")
                || s.wgsl().contains("fs_extract")
                || s.wgsl().contains("fs_blur")
                || matches!(s, BuiltinShader::DepthOnlyMesh3d)
        );
    }
}
