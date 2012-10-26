//! 拖放：源、目标与类型化 payload。预览在帧末绘制。

use std::any::{Any, TypeId};

use spark_core::{Color, Rect, Vec2};
use spark_input::MouseBtn;

use crate::id::WidgetId;
use crate::response::Response;
use crate::state::UiState;
use crate::style::Theme;
use crate::ui::Ui;

/// 拖动中的载荷。
pub struct DragPayload {
    pub(crate) type_id: TypeId,
    pub(crate) data: Box<dyn Any + Send>,
}

impl DragPayload {
    pub fn new<T: Any + Send + 'static>(value: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            data: Box::new(value),
        }
    }

    pub fn is<T: Any + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    pub fn downcast_ref<T: Any + 'static>(&self) -> Option<&T> {
        self.data.downcast_ref::<T>()
    }
}

pub struct DragState {
    pub source: Option<WidgetId>,
    pub payload: Option<DragPayload>,
    pub pointer: Vec2,
    /// 本帧是否在某个目标上松开且被接受。
    pub(crate) drop_accepted: bool,
}

impl std::fmt::Debug for DragState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DragState")
            .field("source", &self.source)
            .field("has_payload", &self.payload.is_some())
            .field("pointer", &self.pointer)
            .field("drop_accepted", &self.drop_accepted)
            .finish()
    }
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            source: None,
            payload: None,
            pointer: Vec2::ZERO,
            drop_accepted: false,
        }
    }
}

impl DragState {
    pub fn begin_frame(&mut self) {
        self.drop_accepted = false;
    }

    pub fn dragging(&self) -> bool {
        self.payload.is_some()
    }

    pub fn take_payload<T: Any + 'static>(&mut self) -> Option<T> {
        let payload = self.payload.take()?;
        self.source = None;
        payload.data.downcast::<T>().ok().map(|b| *b)
    }

    pub fn clear(&mut self) {
        self.source = None;
        self.payload = None;
    }
}

impl Ui<'_> {
    /// 把控件标为拖源。按住超过阈值后开始拖。
    pub fn drag_source<T: Any + Send + 'static>(
        &mut self,
        response: &Response,
        threshold: f32,
        make: impl FnOnce() -> T,
    ) {
        if self.state.drag.payload.is_some() {
            return;
        }
        if response.active && self.input.mouse_down(MouseBtn::Left) {
            let (mx, my) = self.input.mouse_pos();
            let start = response.rect.center();
            let dx = mx - start.x;
            let dy = my - start.y;
            if dx * dx + dy * dy >= threshold * threshold {
                self.state.drag.source = Some(response.id);
                self.state.drag.payload = Some(DragPayload::new(make()));
                self.state.drag.pointer = Vec2::new(mx, my);
                self.capture(response.id);
            }
        }
    }

    /// 放置目标。松开且类型匹配时调用 `on_drop`。
    pub fn drop_target<T: Any + Send + 'static>(
        &mut self,
        response: &Response,
        mut on_drop: impl FnMut(T) -> bool,
    ) -> bool {
        if !response.hovered || !self.input.mouse_released(MouseBtn::Left) {
            return false;
        }
        let Some(payload) = self.state.drag.payload.take() else {
            return false;
        };
        if payload.type_id != TypeId::of::<T>() {
            self.state.drag.payload = Some(payload);
            return false;
        }
        let Ok(value) = payload.data.downcast::<T>() else {
            return false;
        };
        let ok = on_drop(*value);
        if ok {
            self.state.drag.drop_accepted = true;
            self.state.drag.source = None;
        } else {
            // 拒绝则清空，避免悬挂捕获。
            self.state.drag.clear();
        }
        ok
    }
}

pub(crate) fn flush_drag_preview(
    state: &mut UiState,
    draw: &mut spark_renderer::DrawList,
    theme: &Theme,
    input: &spark_input::Input,
) {
    if state.drag.payload.is_none() {
        return;
    }
    let (mx, my) = input.mouse_pos();
    state.drag.pointer = Vec2::new(mx, my);
    if !input.mouse_down(MouseBtn::Left) && !state.drag.drop_accepted {
        // 松开且未被目标接受：取消。
        state.drag.clear();
        return;
    }
    let rect = Rect::new(mx + 12.0, my + 12.0, 36.0, 36.0);
    let mut fill = theme.colors.primary_hover;
    fill.a = 0.85;
    draw.fill_rect(rect, fill);
    draw.fill_rect(
        Rect::new(rect.x - 1.0, rect.y - 1.0, rect.w + 2.0, rect.h + 2.0),
        Color::rgba(1.0, 1.0, 1.0, 0.35),
    );
}
