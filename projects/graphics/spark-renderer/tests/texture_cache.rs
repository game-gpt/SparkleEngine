//! 自 `src/texture_cache.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_renderer::*;

use spark_core::Color;

#[test]
fn hit_skips_decode() {
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    let mut cache = TextureCache::new();
    let mut calls = 0;
    let load = |calls: &mut i32| {
        *calls += 1;
        Ok((1, 1, vec![255, 0, 0, 255]))
    };
    let a = cache.get_or_upload(&mut draw, "icon", || load(&mut calls)).unwrap();
    let b = cache.get_or_upload(&mut draw, "icon", || load(&mut calls)).unwrap();
    assert_eq!(a, b);
    assert_eq!(calls, 1);
    assert_eq!(draw.texture_uploads.len(), 1);
    cache.invalidate("icon");
    let _ = cache.get_or_upload(&mut draw, "icon", || load(&mut calls)).unwrap();
    assert_eq!(calls, 2);
}
