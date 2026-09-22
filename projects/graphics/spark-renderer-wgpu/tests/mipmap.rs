//! 自 `src/mipmap.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_renderer_wgpu::*;

#[test]
fn mip_count_powers_of_two() {
    assert_eq!(mip_level_count(1, 1), 1);
    assert_eq!(mip_level_count(2, 2), 2);
    assert_eq!(mip_level_count(1024, 1024), 11);
    assert_eq!(mip_level_count(1024, 512), 11);
}

#[test]
fn downsample_halves_opaque() {
    let mut src = vec![0u8; 4 * 4 * 4];
    for px in src.chunks_exact_mut(4) {
        px[0] = 200;
        px[1] = 100;
        px[2] = 50;
        px[3] = 255;
    }
    let out = downsample_rgba(&src, 4, 4, 2, 2);
    assert_eq!(out.len(), 2 * 2 * 4);
    assert_eq!(out[0], 200);
    assert_eq!(out[3], 255);
}
