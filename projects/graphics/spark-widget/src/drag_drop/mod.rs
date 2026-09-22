//! 拖放状态与载荷。
//!
//! 指针按下后超过阈值才进入拖动；释放时若悬停在 `drop_target` 上可产生
//! [`UiCommand::Drop`](crate::command::UiCommand::Drop)。

use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use spark_types::Vec2;

use crate::id::WidgetId;

/// 类型擦除的拖放载荷：源控件在开始拖动时注入，目标侧按 `TypeId` 还原。
#[derive(Clone)]
pub struct DragPayload {
    /// 载荷具体类型的 `TypeId`，供 `downcast_ref` 校验。
    pub type_id: TypeId,
    /// 共享所有权的任意可发送数据。
    pub data: Arc<dyn Any + Send + Sync>,
}

impl DragPayload {
    /// 用具体类型 `T` 构造载荷。
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self { type_id: TypeId::of::<T>(), data: Arc::new(value) }
    }

    /// 尝试按类型 `T` 借出载荷引用。
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.data.downcast_ref::<T>()
    }
}

impl std::fmt::Debug for DragPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DragPayload").field("type_id", &self.type_id).finish_non_exhaustive()
    }
}

/// 整棵 UI 共享的拖放会话状态（按下候选 → 拖动中 → 释放 / 取消）。
#[derive(Debug, Default)]
pub struct DragState {
    /// 当前拖动源控件；`None` 表示未进入拖动。
    pub active: Option<WidgetId>,
    /// 拖动中携带的载荷。
    pub payload: Option<DragPayload>,
    /// 按下时的起点（屏幕坐标）。
    pub origin: Vec2,
    /// 指针当前位置。
    pub position: Vec2,
    /// 当前悬停的可放置目标。
    pub hover_target: Option<WidgetId>,
    /// 按下后移动超过该距离才进入拖动。
    pub threshold: f32,
    pending: Option<(WidgetId, Vec2)>,
}

impl DragState {
    /// 记录指针按下：进入「待拖」候选，尚未激活拖动。
    pub fn begin_press(&mut self, source: WidgetId, pos: Vec2) {
        self.pending = Some((source, pos));
        self.origin = pos;
        self.position = pos;
    }

    /// 指针移动：超过阈值则激活拖动并可选写入载荷。返回是否处于拖动中。
    pub fn update_move(&mut self, pos: Vec2, payload: Option<DragPayload>) -> bool {
        self.position = pos;
        if self.active.is_some() {
            return true;
        }
        let Some((source, origin)) = self.pending
        else {
            return false;
        };
        let dx = pos.x - origin.x;
        let dy = pos.y - origin.y;
        let threshold = if self.threshold > 0.0 { self.threshold } else { 6.0 };
        if dx * dx + dy * dy >= threshold * threshold {
            self.active = Some(source);
            if payload.is_some() {
                self.payload = payload;
            }
            self.pending = None;
            true
        }
        else {
            false
        }
    }

    /// 结束拖动并取出 `(源, 载荷, 悬停目标)`；无活动拖动时返回 `None`。
    pub fn end(&mut self) -> Option<(WidgetId, Option<DragPayload>, Option<WidgetId>)> {
        self.pending = None;
        let source = self.active.take()?;
        let payload = self.payload.take();
        let target = self.hover_target.take();
        Some((source, payload, target))
    }

    /// 取消拖动：清除候选、活动源、载荷与悬停目标。
    pub fn cancel(&mut self) {
        self.pending = None;
        self.active = None;
        self.payload = None;
        self.hover_target = None;
    }

    /// 是否已越过阈值、处于拖动中。
    pub fn is_dragging(&self) -> bool {
        self.active.is_some()
    }
}
