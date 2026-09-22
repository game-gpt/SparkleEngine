//! Spark 调试框架：叠加绘制、帧统计与检查器钩子。
//! 不含游戏专用面板或远程调试协议产品。

#![warn(missing_docs)]
pub use spark_types::{Color, Rect, Vec2};
use spark_renderer::DrawList;

/// 单帧性能与计数快照。
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub dt_seconds: f32,
    pub fps: f32,
    pub draw_quads: u32,
    pub draw_texts: u32,
    pub entities: u64,
}

impl FrameStats {
    pub fn from_dt(dt_seconds: f32) -> Self {
        let fps = if dt_seconds > 0.0 { 1.0 / dt_seconds } else { 0.0 };
        Self { dt_seconds, fps, ..Default::default() }
    }
}

/// 调试叠加图元（屏幕或世界坐标由调用方约定）。
#[derive(Debug, Clone)]
pub enum DebugPrim {
    Line { a: Vec2, b: Vec2, color: Color, thickness: f32 },
    Rect { rect: Rect, color: Color, filled: bool },
    Text { pos: Vec2, size: f32, color: Color, text: String },
}

/// 本帧调试绘制缓冲。每帧 `clear` 后由系统写入，再 `flush` 到 `DrawList`。
#[derive(Debug, Default)]
pub struct DebugDraw {
    prims: Vec<DebugPrim>,
    enabled: bool,
}

impl DebugDraw {
    pub fn new() -> Self {
        Self { prims: Vec::new(), enabled: true }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn clear(&mut self) {
        self.prims.clear();
    }

    pub fn line(&mut self, a: Vec2, b: Vec2, color: Color, thickness: f32) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Line { a, b, color, thickness });
    }

    pub fn rect_outline(&mut self, rect: Rect, color: Color) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Rect { rect, color, filled: false });
    }

    pub fn rect_filled(&mut self, rect: Rect, color: Color) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Rect { rect, color, filled: true });
    }

    pub fn text(&mut self, pos: Vec2, size: f32, color: Color, text: impl Into<String>) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Text { pos, size, color, text: text.into() });
    }

    pub fn prims(&self) -> &[DebugPrim] {
        &self.prims
    }

    /// 将调试图元刷入绘制列表。线用细矩形近似（无独立线段 GPU 路径时）。
    pub fn flush(&self, draw: &mut DrawList) {
        if !self.enabled {
            return;
        }
        for prim in &self.prims {
            match prim {
                DebugPrim::Line { a, b, color, thickness } => {
                    flush_line(draw, *a, *b, *color, *thickness);
                }
                DebugPrim::Rect { rect, color, filled } => {
                    if *filled {
                        draw.fill_rect(*rect, *color);
                    }
                    else {
                        flush_rect_outline(draw, *rect, *color);
                    }
                }
                DebugPrim::Text { pos, size, color, text } => {
                    draw.text(pos.x, pos.y, *size, *color, text);
                }
            }
        }
    }
}

fn flush_line(draw: &mut DrawList, a: Vec2, b: Vec2, color: Color, thickness: f32) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-4);
    let t = thickness.max(1.0);
    // 轴对齐近似：水平/垂直占优时用细矩形；斜线拆成采样点块。
    if dx.abs() >= dy.abs() {
        let y = (a.y + b.y) * 0.5 - t * 0.5;
        let x = a.x.min(b.x);
        draw.fill_rect(Rect::new(x, y, len, t), color);
    }
    else {
        let x = (a.x + b.x) * 0.5 - t * 0.5;
        let y = a.y.min(b.y);
        draw.fill_rect(Rect::new(x, y, t, len), color);
    }
}

fn flush_rect_outline(draw: &mut DrawList, rect: Rect, color: Color) {
    let t = 1.0;
    draw.fill_rect(Rect::new(rect.x, rect.y, rect.w, t), color);
    draw.fill_rect(Rect::new(rect.x, rect.y + rect.h - t, rect.w, t), color);
    draw.fill_rect(Rect::new(rect.x, rect.y, t, rect.h), color);
    draw.fill_rect(Rect::new(rect.x + rect.w - t, rect.y, t, rect.h), color);
}

/// 检查器可展示的只读字段。
#[derive(Debug, Clone)]
pub struct InspectField {
    pub name: &'static str,
    pub value: String,
}

/// 检查器节点：游戏或引擎向其挂载可观测对象。
#[derive(Debug, Clone)]
pub struct InspectNode {
    pub label: String,
    pub fields: Vec<InspectField>,
}

/// 检查器钩子：收集本帧可观测树。实现方可对接未来 ECS 查询。
pub trait Inspector {
    fn collect(&self) -> Vec<InspectNode>;
}

/// 空检查器。
#[derive(Debug, Default)]
pub struct NopInspector;

impl Inspector for NopInspector {
    fn collect(&self) -> Vec<InspectNode> {
        Vec::new()
    }
}

/// 调试会话：绘制 + 统计 + 可选检查器。
pub struct DebugSession {
    pub draw: DebugDraw,
    pub stats: FrameStats,
    inspector: Box<dyn Inspector + Send>,
}

impl Default for DebugSession {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugSession {
    pub fn new() -> Self {
        Self { draw: DebugDraw::new(), stats: FrameStats::default(), inspector: Box::new(NopInspector) }
    }

    pub fn set_inspector<I: Inspector + Send + 'static>(&mut self, inspector: I) {
        self.inspector = Box::new(inspector);
    }

    pub fn begin_frame(&mut self, dt_seconds: f32) {
        self.draw.clear();
        self.stats = FrameStats::from_dt(dt_seconds);
    }

    pub fn inspect(&self) -> Vec<InspectNode> {
        self.inspector.collect()
    }

    /// 在屏角画出 FPS 等简要叠加。
    pub fn overlay_stats(&mut self, origin: Vec2) {
        let text = format!("fps {:.0}  dt {:.2}ms  ents {}", self.stats.fps, self.stats.dt_seconds * 1000.0, self.stats.entities);
        self.draw.text(origin, 16.0, Color::rgb(0.85, 0.95, 0.75), text);
    }

    pub fn flush_draw(&self, draw: &mut DrawList) {
        self.draw.flush(draw);
    }
}
