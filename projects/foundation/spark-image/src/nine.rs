//! 九宫格（九切片）布局：四角固定，边与中心按模式拉伸或平铺。

use spark_core::{ErrorArg, Rect, SparkError, codes};

use crate::{PixelImage, validate_region};

/// 九宫格边距（相对源矩形内侧，像素）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Margin {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Margin {
    pub const fn uniform(v: f32) -> Self {
        Self { left: v, right: v, top: v, bottom: v }
    }

    pub const fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self { left, right, top, bottom }
    }
}

/// 中心与边的填充方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NineSliceMode {
    /// 线性拉伸（默认，适合纯色 / 渐变边框）。
    #[default]
    Stretch,
    /// 按源片尺寸重复平铺（适合点九图案纹理）。
    Tile,
}

/// 九宫格定义：图集中的源区 + 边距。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NineSlice {
    pub source: Rect,
    pub margin: Margin,
    pub mode: NineSliceMode,
}

/// 单个九宫格输出四边形：源像素区 → 目标区。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NineQuad {
    pub src: Rect,
    pub dst: Rect,
}

impl NineSlice {
    pub fn new(source: Rect, margin: Margin) -> Self {
        Self { source, margin, mode: NineSliceMode::Stretch }
    }

    pub fn with_mode(mut self, mode: NineSliceMode) -> Self {
        self.mode = mode;
        self
    }

    /// 校验源区落在纹理尺寸内，且边距合法。
    pub fn validate(&self, width: u32, height: u32) -> Result<(), SparkError> {
        validate_region(width, height, self.source)?;
        self.validate_margin()
    }

    /// 过渡：相对 [`PixelImage`] 校验（请改用 [`Self::validate`]）。
    pub fn validate_image(&self, image: &PixelImage) -> Result<(), SparkError> {
        self.validate(image.width(), image.height())
    }

    fn validate_margin(&self) -> Result<(), SparkError> {
        let m = self.margin;
        if m.left < 0.0 || m.right < 0.0 || m.top < 0.0 || m.bottom < 0.0 {
            return Err(SparkError::new(codes::image_nine_margin_invalid()).arg("reason", ErrorArg::String("negative".into())));
        }
        if m.left + m.right > self.source.w + 1e-3 || m.top + m.bottom > self.source.h + 1e-3 {
            return Err(SparkError::new(codes::image_nine_margin_invalid())
                .arg("reason", ErrorArg::String("exceeds_source".into()))
                .arg("source_w", ErrorArg::Float(self.source.w as f64))
                .arg("source_h", ErrorArg::Float(self.source.h as f64)));
        }
        Ok(())
    }

    /// 将源图切成最多 9 块，映射到 `dest`。
    ///
    /// 目标过小时优先保证四角，边与中心宽度/高度可被压缩到 0（对应块省略）。
    pub fn layout(&self, dest: Rect) -> Result<Vec<NineQuad>, SparkError> {
        self.validate_margin()?;
        if dest.w <= 0.0 || dest.h <= 0.0 {
            return Err(SparkError::new(codes::image_dest_invalid())
                .arg("w", ErrorArg::Float(dest.w as f64))
                .arg("h", ErrorArg::Float(dest.h as f64)));
        }

        let sx = self.source.x;
        let sy = self.source.y;
        let sw = self.source.w;
        let sh = self.source.h;
        let ml = self.margin.left;
        let mr = self.margin.right;
        let mt = self.margin.top;
        let mb = self.margin.bottom;

        // 目标边：不超过源边距，且左右/上下之和不超过目标尺寸。
        let dl = ml.min(dest.w * 0.5);
        let dr = mr.min(dest.w - dl);
        let dt = mt.min(dest.h * 0.5);
        let db = mb.min(dest.h - dt);
        let dcw = (dest.w - dl - dr).max(0.0);
        let dch = (dest.h - dt - db).max(0.0);

        let scw = (sw - ml - mr).max(0.0);
        let sch = (sh - mt - mb).max(0.0);

        // 源 3×3 片（宽或高为 0 的列/行稍后跳过）
        let src_x = [sx, sx + ml, sx + ml + scw];
        let src_y = [sy, sy + mt, sy + mt + sch];
        let src_w = [ml, scw, mr];
        let src_h = [mt, sch, mb];

        let dst_x = [dest.x, dest.x + dl, dest.x + dl + dcw];
        let dst_y = [dest.y, dest.y + dt, dest.y + dt + dch];
        let dst_w = [dl, dcw, dr];
        let dst_h = [dt, dch, db];

        let mut out = Vec::with_capacity(9);
        for row in 0..3 {
            for col in 0..3 {
                let sw_i = src_w[col];
                let sh_i = src_h[row];
                let dw_i = dst_w[col];
                let dh_i = dst_h[row];
                if sw_i <= 1e-6 || sh_i <= 1e-6 || dw_i <= 1e-6 || dh_i <= 1e-6 {
                    continue;
                }
                let src = Rect::new(src_x[col], src_y[row], sw_i, sh_i);
                match self.mode {
                    NineSliceMode::Stretch => {
                        out.push(NineQuad { src, dst: Rect::new(dst_x[col], dst_y[row], dw_i, dh_i) });
                    }
                    NineSliceMode::Tile => {
                        push_tiled(&mut out, src, Rect::new(dst_x[col], dst_y[row], dw_i, dh_i));
                    }
                }
            }
        }
        Ok(out)
    }
}

fn push_tiled(out: &mut Vec<NineQuad>, src: Rect, dest: Rect) {
    let mut y = dest.y;
    while y < dest.y + dest.h - 1e-6 {
        let h = src.h.min(dest.y + dest.h - y);
        let mut x = dest.x;
        while x < dest.x + dest.w - 1e-6 {
            let w = src.w.min(dest.x + dest.w - x);
            // 裁切源：末片可能小于完整格
            let src_piece = Rect::new(src.x, src.y, w.min(src.w), h.min(src.h));
            out.push(NineQuad { src: src_piece, dst: Rect::new(x, y, w, h) });
            x += src.w;
        }
        y += src.h;
    }
}
