//! 精灵与网格图集裁切。

use spark_core::{ErrorArg, Rect, SparkError, Vec2, codes};

use crate::{validate_region, PixelImage};

/// 图集中的精灵：像素源矩形 + 归一化轴心（相对源矩形，默认中心 `(0.5, 0.5)`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    pub region: Rect,
    pub pivot: Vec2,
}

impl Sprite {
    pub fn new(region: Rect) -> Self {
        Self {
            region,
            pivot: Vec2::new(0.5, 0.5),
        }
    }

    pub fn with_pivot(mut self, pivot: Vec2) -> Self {
        self.pivot = pivot;
        self
    }

    /// 校验精灵落在图像内。
    pub fn validate(&self, image: &PixelImage) -> Result<(), SparkError> {
        validate_region(image.width(), image.height(), self.region)
    }

    /// 源矩形对应的归一化 UV。
    pub fn uv(&self, image: &PixelImage) -> Result<Rect, SparkError> {
        image.uv_rect(self.region)
    }

    /// 以轴心对齐到目标点时的目标矩形（`dst_size` 为绘制宽高）。
    pub fn dest_rect(&self, anchor: Vec2, dst_size: Vec2) -> Rect {
        Rect::new(
            anchor.x - self.pivot.x * dst_size.x,
            anchor.y - self.pivot.y * dst_size.y,
            dst_size.x,
            dst_size.y,
        )
    }
}

/// 等分网格精灵表（从整图切格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteSheet {
    pub columns: u32,
    pub rows: u32,
    /// 每格像素宽。
    pub cell_w: u32,
    /// 每格像素高。
    pub cell_h: u32,
    /// 表内左边距。
    pub margin_x: u32,
    pub margin_y: u32,
    /// 格间距。
    pub spacing_x: u32,
    pub spacing_y: u32,
}

impl SpriteSheet {
    pub fn grid(columns: u32, rows: u32, cell_w: u32, cell_h: u32) -> Self {
        Self {
            columns,
            rows,
            cell_w,
            cell_h,
            margin_x: 0,
            margin_y: 0,
            spacing_x: 0,
            spacing_y: 0,
        }
    }

    pub fn cell_count(&self) -> u32 {
        self.columns.saturating_mul(self.rows)
    }

    /// 按行列取精灵（列先行后，原点左上）。
    pub fn sprite_at(&self, col: u32, row: u32) -> Result<Sprite, SparkError> {
        if col >= self.columns || row >= self.rows {
            return Err(SparkError::new(codes::image_sprite_out_of_bounds())
                .arg("col", ErrorArg::Unsigned(col as u64))
                .arg("row", ErrorArg::Unsigned(row as u64))
                .arg("columns", ErrorArg::Unsigned(self.columns as u64))
                .arg("rows", ErrorArg::Unsigned(self.rows as u64)));
        }
        let x = self.margin_x + col * (self.cell_w + self.spacing_x);
        let y = self.margin_y + row * (self.cell_h + self.spacing_y);
        Ok(Sprite::new(Rect::new(
            x as f32,
            y as f32,
            self.cell_w as f32,
            self.cell_h as f32,
        )))
    }

    /// 按线性下标取精灵（行主序）。
    pub fn sprite_index(&self, index: u32) -> Result<Sprite, SparkError> {
        if self.columns == 0 {
            return Err(SparkError::new(codes::image_sprite_grid_invalid())
                .arg("reason", ErrorArg::String("zero_columns".into())));
        }
        let col = index % self.columns;
        let row = index / self.columns;
        self.sprite_at(col, row)
    }

    /// 从图像尺寸推断格大小（无边距无间距时）。
    pub fn from_image(image: &PixelImage, columns: u32, rows: u32) -> Result<Self, SparkError> {
        if columns == 0 || rows == 0 {
            return Err(SparkError::new(codes::image_sprite_grid_invalid())
                .arg("reason", ErrorArg::String("non_positive_grid".into()))
                .arg("columns", ErrorArg::Unsigned(columns as u64))
                .arg("rows", ErrorArg::Unsigned(rows as u64)));
        }
        if image.width() % columns != 0 || image.height() % rows != 0 {
            return Err(SparkError::new(codes::image_sprite_grid_invalid())
                .arg("reason", ErrorArg::String("not_divisible".into()))
                .arg("img_w", ErrorArg::Unsigned(image.width() as u64))
                .arg("img_h", ErrorArg::Unsigned(image.height() as u64))
                .arg("columns", ErrorArg::Unsigned(columns as u64))
                .arg("rows", ErrorArg::Unsigned(rows as u64)));
        }
        Ok(Self::grid(
            columns,
            rows,
            image.width() / columns,
            image.height() / rows,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Color;

    #[test]
    fn sheet_index() {
        let img = PixelImage::solid(64, 32, Color::rgb(0.0, 1.0, 0.0)).unwrap();
        let sheet = SpriteSheet::from_image(&img, 4, 2).unwrap();
        let s = sheet.sprite_index(5).unwrap();
        assert_eq!(s.region.x, 16.0);
        assert_eq!(s.region.y, 16.0);
        assert_eq!(s.region.w, 16.0);
        assert_eq!(s.region.h, 16.0);
    }
}
