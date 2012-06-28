//! 字体装载、字形栅格化与 CPU 侧图集。
//!
//! 本 crate **不**碰 GPU：`spark-renderer-wgpu` 只消费 `GlyphCache` 的图集字节与 UV，自行上传纹理。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fontdue::{Font, FontSettings};
use spark_core::{ErrorArg, SparkError, codes};

/// 单个已栅格字形在图集中的布局信息。
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub width: f32,
    pub height: f32,
    pub bearing_y: f32,
    pub advance: f32,
}

/// CPU 字形图集：按需栅格化，脏标记供渲染侧上传。
pub struct GlyphCache {
    font: Font,
    atlas: Vec<u8>,
    atlas_w: u32,
    atlas_h: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
    glyphs: HashMap<(char, u32), GlyphInfo>,
    dirty: bool,
}

impl GlyphCache {
    /// 从本机常见中西文字体路径装载；失败则返回错误（不静默降级成空白字）。
    pub fn load_system() -> Result<Self, SparkError> {
        for path in system_font_candidates() {
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                    tracing::info!(path = %path.display(), "已加载系统字体");
                    return Ok(Self::new(font));
                }
            }
        }
        Err(SparkError::new(codes::font_not_found()).arg(
            "tried",
            ErrorArg::String(Arc::from("msyh,simhei,arial")),
        ))
    }

    /// 从字节流装载（测试 / 打包字体）。
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, SparkError> {
        let font = Font::from_bytes(bytes.as_ref(), FontSettings::default()).map_err(|e| {
            SparkError::new(codes::font_parse())
                .arg("bytes", ErrorArg::Unsigned(bytes.as_ref().len() as u64))
                .caused_by(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        Ok(Self::new(font))
    }

    /// 从文件路径装载。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SparkError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| {
            SparkError::new(codes::io())
                .arg("path", ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref())))
                .arg("op", ErrorArg::String(Arc::from("read")))
                .caused_by(e)
        })?;
        Self::from_bytes(bytes)
    }

    fn new(font: Font) -> Self {
        let atlas_w = 1024u32;
        let atlas_h = 1024u32;
        Self {
            font,
            atlas: vec![0u8; (atlas_w * atlas_h) as usize],
            atlas_w,
            atlas_h,
            cursor_x: 1,
            cursor_y: 1,
            row_h: 0,
            glyphs: HashMap::new(),
            dirty: true,
        }
    }

    pub fn atlas_size(&self) -> (u32, u32) {
        (self.atlas_w, self.atlas_h)
    }

    pub fn atlas_bytes(&self) -> &[u8] {
        &self.atlas
    }

    /// 取出并清除脏标记。返回 `true` 表示图集自上次上传后有改动。
    pub fn take_dirty(&mut self) -> bool {
        let d = self.dirty;
        self.dirty = false;
        d
    }

    pub fn glyph(&mut self, ch: char, px: f32) -> Option<&GlyphInfo> {
        let key = (ch, px.round() as u32);
        if self.glyphs.contains_key(&key) {
            return self.glyphs.get(&key);
        }
        let (metrics, bitmap) = self.font.rasterize(ch, px);
        let gw = metrics.width.max(1) as u32;
        let gh = metrics.height.max(1) as u32;
        if self.cursor_x + gw + 1 >= self.atlas_w {
            self.cursor_x = 1;
            self.cursor_y += self.row_h + 1;
            self.row_h = 0;
        }
        if self.cursor_y + gh + 1 >= self.atlas_h {
            tracing::warn!("字形图集已满，跳过字符");
            return None;
        }
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let src = bitmap[row * metrics.width + col];
                let dx = self.cursor_x as usize + col;
                let dy = self.cursor_y as usize + row;
                self.atlas[dy * self.atlas_w as usize + dx] = src;
            }
        }
        let uv_min = [
            self.cursor_x as f32 / self.atlas_w as f32,
            self.cursor_y as f32 / self.atlas_h as f32,
        ];
        let uv_max = [
            (self.cursor_x + gw) as f32 / self.atlas_w as f32,
            (self.cursor_y + gh) as f32 / self.atlas_h as f32,
        ];
        let info = GlyphInfo {
            uv_min,
            uv_max,
            width: metrics.width as f32,
            height: metrics.height as f32,
            bearing_y: metrics.ymin as f32,
            advance: metrics.advance_width,
        };
        self.cursor_x += gw + 1;
        self.row_h = self.row_h.max(gh);
        self.dirty = true;
        self.glyphs.insert(key, info);
        self.glyphs.get(&key)
    }

    pub fn measure(&mut self, text: &str, px: f32) -> f32 {
        let mut w = 0.0f32;
        for ch in text.chars() {
            if let Some(g) = self.glyph(ch, px) {
                w += g.advance;
            }
        }
        w
    }
}

fn system_font_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    // 用环境变量拼系统字体目录，避免写死盘符。
    if let Ok(windir) = std::env::var("WINDIR") {
        let fonts = PathBuf::from(windir).join("Fonts");
        for name in [
            "msyh.ttc",
            "msyhbd.ttc",
            "simhei.ttf",
            "arial.ttf",
            "segoeui.ttf",
        ] {
            out.push(fonts.join(name));
        }
    }
    // 非 Windows 常见路径（有则试）
    for path in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ] {
        out.push(PathBuf::from(path));
    }
    out
}
