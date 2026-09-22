//! UI 调试观察。
//!
//! 提供布局快照、事件轨迹与高亮目标，供 Studio / 调试覆盖层读取；
//! 不改变树拓扑或交互语义。

use spark_types::Rect;

use crate::{id::WidgetId, tree::WidgetTree};

/// 单个节点的布局调试快照（几何 + 结构）。
#[derive(Debug, Clone)]
pub struct LayoutDump {
    /// 节点 ID。
    pub id: WidgetId,
    /// 种类的调试字符串（`Debug` 格式）。
    pub kind: String,
    /// 稳定键（若有）。
    pub key: Option<String>,
    /// 边框盒（含 padding 外缘）。
    pub rect: Rect,
    /// 内容盒。
    pub content_rect: Rect,
    /// 裁剪矩形（滚动视口等）。
    pub clip_rect: Option<Rect>,
    /// 测量阶段得到的期望尺寸。
    pub desired: crate::layout::Size2,
    /// 直接子节点 ID。
    pub children: Vec<WidgetId>,
}

/// 一条事件调试轨迹。
#[derive(Debug, Clone)]
pub struct UiEventTrace {
    /// 事件种类短名（如 `"PointerDown"`）。
    pub kind: &'static str,
    /// 命中或派发目标；未命中时为 `None`。
    pub target: Option<WidgetId>,
    /// 附加细节（坐标、按键等可读摘要）。
    pub detail: String,
}

/// UI 检查器：高亮、布局 dump、环形事件轨迹缓冲。
#[derive(Debug, Default)]
pub struct UiInspector {
    /// 当前高亮的控件（调试描边用）。
    pub highlight: Option<WidgetId>,
    traces: Vec<UiEventTrace>,
    max_traces: usize,
}

impl UiInspector {
    /// 创建默认检查器（轨迹上限 128 条）。
    pub fn new() -> Self {
        Self { highlight: None, traces: Vec::new(), max_traces: 128 }
    }

    /// 透传 runtime 树引用，便于调试 UI 与业务树共用同一实例。
    pub fn tree<'a>(&self, runtime_tree: &'a WidgetTree) -> &'a WidgetTree {
        runtime_tree
    }

    /// 设置高亮目标。
    pub fn highlight(&mut self, id: WidgetId) {
        self.highlight = Some(id);
    }

    /// 清除高亮。
    pub fn clear_highlight(&mut self) {
        self.highlight = None;
    }

    /// 导出指定节点的布局快照；节点不存在时返回 `None`。
    pub fn dump_layout(&self, tree: &WidgetTree, id: WidgetId) -> Option<LayoutDump> {
        let node = tree.node(id)?;
        Some(LayoutDump {
            id,
            kind: format!("{:?}", node.kind),
            key: node.key.clone(),
            rect: node.computed.rect,
            content_rect: node.computed.content_rect,
            clip_rect: node.computed.clip_rect,
            desired: node.computed.desired,
            children: node.children.clone(),
        })
    }

    /// 追加一条事件轨迹；超出上限时丢弃最旧记录。
    pub fn push_trace(&mut self, kind: &'static str, target: Option<WidgetId>, detail: impl Into<String>) {
        if self.max_traces == 0 {
            return;
        }
        self.traces.push(UiEventTrace { kind, target, detail: detail.into() });
        if self.traces.len() > self.max_traces {
            let overflow = self.traces.len() - self.max_traces;
            self.traces.drain(0..overflow);
        }
    }

    /// 只读访问轨迹缓冲。
    pub fn traces(&self) -> &[UiEventTrace] {
        &self.traces
    }

    /// 清空全部轨迹。
    pub fn clear_traces(&mut self) {
        self.traces.clear();
    }
}
