//! UI 调试观察。

use spark_types::Rect;

use crate::{id::WidgetId, tree::WidgetTree};

#[derive(Debug, Clone)]
pub struct LayoutDump {
    pub id: WidgetId,
    pub kind: String,
    pub key: Option<String>,
    pub rect: Rect,
    pub content_rect: Rect,
    pub clip_rect: Option<Rect>,
    pub desired: crate::layout::Size2,
    pub children: Vec<WidgetId>,
}

#[derive(Debug, Clone)]
pub struct UiEventTrace {
    pub kind: &'static str,
    pub target: Option<WidgetId>,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct UiInspector {
    pub highlight: Option<WidgetId>,
    traces: Vec<UiEventTrace>,
    max_traces: usize,
}

impl UiInspector {
    pub fn new() -> Self {
        Self { highlight: None, traces: Vec::new(), max_traces: 128 }
    }

    pub fn tree<'a>(&self, runtime_tree: &'a WidgetTree) -> &'a WidgetTree {
        runtime_tree
    }

    pub fn highlight(&mut self, id: WidgetId) {
        self.highlight = Some(id);
    }

    pub fn clear_highlight(&mut self) {
        self.highlight = None;
    }

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

    pub fn traces(&self) -> &[UiEventTrace] {
        &self.traces
    }

    pub fn clear_traces(&mut self) {
        self.traces.clear();
    }
}
