//! 像素区域校验（相对纹理宽高，不持有像素）。

use spark_core::{ErrorArg, Rect, SparkError, codes};

/// 校验 `region` 落在 `[0, width] × [0, height]` 内且宽高为正。
pub(crate) fn validate_region(img_w: u32, img_h: u32, region: Rect) -> Result<(), SparkError> {
    if region.w <= 0.0 || region.h <= 0.0 {
        return Err(SparkError::new(codes::image_region_invalid())
            .arg("w", ErrorArg::Float(region.w as f64))
            .arg("h", ErrorArg::Float(region.h as f64)));
    }
    if region.x < 0.0 || region.y < 0.0 || region.x + region.w > img_w as f32 + 1e-3 || region.y + region.h > img_h as f32 + 1e-3 {
        return Err(SparkError::new(codes::image_region_out_of_bounds())
            .arg("x", ErrorArg::Float(region.x as f64))
            .arg("y", ErrorArg::Float(region.y as f64))
            .arg("w", ErrorArg::Float(region.w as f64))
            .arg("h", ErrorArg::Float(region.h as f64))
            .arg("img_w", ErrorArg::Unsigned(img_w as u64))
            .arg("img_h", ErrorArg::Unsigned(img_h as u64)));
    }
    Ok(())
}
