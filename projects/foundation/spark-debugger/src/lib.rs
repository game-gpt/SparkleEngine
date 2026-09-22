//! Spark 调试框架：叠加绘制、帧统计与检查器钩子。
//! 不含游戏专用面板或远程调试协议产品。

#![deny(missing_docs)]
pub use spark_types::{Color, Rect, Vec2};
use spark_renderer::DrawList;

/// 单帧性能与计数快照。
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    /// 上一帧耗时（秒）；由宿主传入墙钟或仿真步进。
    pub dt_seconds: f32,
    /// 瞬时帧率近似：`1.0 / dt_seconds`；`dt_seconds <= 0` 时为 `0`。
    pub fps: f32,
    /// 本帧提交的四边形绘制次数（宿主填充）。
    pub draw_quads: u32,
    /// 本帧提交的文本绘制次数（宿主填充）。
    pub draw_texts: u32,
    /// 本帧可观测实体数量（宿主填充，单位为个数）。
    pub entities: u64,
}

impl FrameStats {
    /// 仅根据帧间隔构造；`fps` 由 `dt_seconds` 推导，其余计数为 0。
    pub fn from_dt(dt_seconds: f32) -> Self {
        let fps = if dt_seconds > 0.0 { 1.0 / dt_seconds } else { 0.0 };
        Self { dt_seconds, fps, ..Default::default() }
    }
}

/// 调试叠加图元（屏幕或世界坐标由调用方约定）。
#[derive(Debug, Clone)]
pub enum DebugPrim {
    /// 线段：从 `a` 到 `b`。
    Line {
        /// 起点。
        a: Vec2,
        /// 终点。
        b: Vec2,
        /// 线颜色（RGBA，分量通常在 0..=1）。
        color: Color,
        /// 线宽（像素或世界单位，与坐标约定一致）；`flush` 时至少按 1.0 绘制。
        thickness: f32,
    },
    /// 轴对齐矩形轮廓或填充。
    Rect {
        /// 矩形区域。
        rect: Rect,
        /// 描边/填充颜色。
        color: Color,
        /// `true` 填充整块；`false` 仅 1 像素宽轮廓。
        filled: bool,
    },
    /// 调试文本标签。
    Text {
        /// 文本锚点（通常为左上或基线，取决于渲染器）。
        pos: Vec2,
        /// 字号（像素高度量级）。
        size: f32,
        /// 文字颜色。
        color: Color,
        /// 显示字符串（可含临时诊断信息）。
        text: String,
    },
}

/// 本帧调试绘制缓冲。每帧 `clear` 后由系统写入，再 `flush` 到 `DrawList`。
#[derive(Debug, Default)]
pub struct DebugDraw {
    prims: Vec<DebugPrim>,
    enabled: bool,
}

impl DebugDraw {
    /// 新建缓冲；默认启用绘制。
    pub fn new() -> Self {
        Self { prims: Vec::new(), enabled: true }
    }

    /// 开关叠加；关闭后写入与 `flush` 均为空操作，已有图元仍保留直至 `clear`。
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 当前是否接受新图元并参与 `flush`。
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 清空本帧图元列表（不改变 `enabled`）。
    pub fn clear(&mut self) {
        self.prims.clear();
    }

    /// 追加线段；已禁用时忽略。
    pub fn line(&mut self, a: Vec2, b: Vec2, color: Color, thickness: f32) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Line { a, b, color, thickness });
    }

    /// 追加矩形轮廓（`filled = false`）；已禁用时忽略。
    pub fn rect_outline(&mut self, rect: Rect, color: Color) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Rect { rect, color, filled: false });
    }

    /// 追加实心矩形；已禁用时忽略。
    pub fn rect_filled(&mut self, rect: Rect, color: Color) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Rect { rect, color, filled: true });
    }

    /// 追加文本；已禁用时忽略。`size` 为字号，`text` 转为自有 `String`。
    pub fn text(&mut self, pos: Vec2, size: f32, color: Color, text: impl Into<String>) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Text { pos, size, color, text: text.into() });
    }

    /// 只读查看本帧已登记图元。
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
    /// 字段名（静态标签，如 `"hp"`）。
    pub name: &'static str,
    /// 已格式化的显示值（非权威状态，仅供 UI）。
    pub value: String,
}

/// 检查器节点：游戏或引擎向其挂载可观测对象。
#[derive(Debug, Clone)]
pub struct InspectNode {
    /// 节点标题（实体名、资源名等）。
    pub label: String,
    /// 该节点下的只读字段列表。
    pub fields: Vec<InspectField>,
}

/// 检查器钩子：收集本帧可观测树。实现方可对接未来 ECS 查询。
pub trait Inspector {
    /// 返回本帧要展示的节点列表；可为空。
    fn collect(&self) -> Vec<InspectNode>;
}

/// 空检查器：始终返回空树，作默认占位。
#[derive(Debug, Default)]
pub struct NopInspector;

impl Inspector for NopInspector {
    fn collect(&self) -> Vec<InspectNode> {
        Vec::new()
    }
}

/// 调试会话：绘制 + 统计 + 可选检查器。
pub struct DebugSession {
    /// 本帧叠加绘制缓冲。
    pub draw: DebugDraw,
    /// 本帧性能/计数快照。
    pub stats: FrameStats,
    inspector: Box<dyn Inspector + Send>,
}

impl Default for DebugSession {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugSession {
    /// 新建会话：启用绘制、零统计、[`NopInspector`]。
    pub fn new() -> Self {
        Self { draw: DebugDraw::new(), stats: FrameStats::default(), inspector: Box::new(NopInspector) }
    }

    /// 替换检查器实现；旧实现被丢弃。
    pub fn set_inspector<I: Inspector + Send + 'static>(&mut self, inspector: I) {
        self.inspector = Box::new(inspector);
    }

    /// 帧初：清空叠加，并按 `dt_seconds`（秒）重置 [`FrameStats`]。
    pub fn begin_frame(&mut self, dt_seconds: f32) {
        self.draw.clear();
        self.stats = FrameStats::from_dt(dt_seconds);
    }

    /// 向当前检查器拉取本帧可观测树。
    pub fn inspect(&self) -> Vec<InspectNode> {
        self.inspector.collect()
    }

    /// 在屏角画出 FPS 等简要叠加。
    pub fn overlay_stats(&mut self, origin: Vec2) {
        let text = format!("fps {:.0}  dt {:.2}ms  ents {}", self.stats.fps, self.stats.dt_seconds * 1000.0, self.stats.entities);
        self.draw.text(origin, 16.0, Color::rgb(0.85, 0.95, 0.75), text);
    }

    /// 将 [`Self::draw`] 刷入宿主 [`DrawList`]。
    pub fn flush_draw(&self, draw: &mut DrawList) {
        self.draw.flush(draw);
    }
}
