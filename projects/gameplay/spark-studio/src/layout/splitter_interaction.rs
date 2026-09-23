//! 分栏命中与拖拽（在 \`app\` 帧循环中处理，不经过 Widget 命令）。

use spark_input::Input;

use super::DockLayoutState;

/// 可拖拽的分栏轴。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterAxis {
    /// 层级与中栏之间。
    Hierarchy,
    /// 中栏与检查器之间。
    Inspector,
    /// 中栏视口与底栏之间。
    Bottom,
}

/// 进行中的分栏拖拽。
#[derive(Debug, Clone, Copy)]
pub struct SplitterDrag {
    /// 拖拽轴。
    pub axis: SplitterAxis,
    /// 按下时指针位置（x 或 y）。
    pub anchor: f32,
    /// 按下时的层级宽度。
    pub start_hierarchy: f32,
    /// 按下时的检查器宽度。
    pub start_inspector: f32,
    /// 按下时的底栏高度。
    pub start_bottom: f32,
}

/// 根据当前布局计算三条分栏线的屏幕位置。
pub fn splitter_positions(screen_w: f32, screen_h: f32, dock: DockLayoutState) -> (f32, f32, f32) {
    let main_top = dock.main_top();
    let main_h = dock.main_height(screen_h);
    let left_x = dock.hierarchy_effective();
    let right_x = screen_w - dock.inspector_effective();
    let bottom_h = dock.bottom_effective();
    let bottom_y = if bottom_h > 0.0 { main_top + main_h - bottom_h - DockLayoutState::SPLITTER_THICKNESS } else { main_top + main_h };
    (left_x, right_x, bottom_y)
}

/// 命中测试：指针是否落在分栏热区。
pub fn hit_splitter(input: &Input, screen_w: f32, screen_h: f32, dock: DockLayoutState) -> Option<SplitterAxis> {
    let (mx, my) = input.mouse_pos();
    let hit = DockLayoutState::SPLITTER_HIT_HALF;
    let (left_x, right_x, bottom_y) = splitter_positions(screen_w, screen_h, dock);

    if !dock.hierarchy_collapsed && (mx - left_x).abs() <= hit && my >= dock.main_top() && my <= dock.main_top() + dock.main_height(screen_h) {
        return Some(SplitterAxis::Hierarchy);
    }
    if !dock.inspector_collapsed && (mx - right_x).abs() <= hit && my >= dock.main_top() && my <= dock.main_top() + dock.main_height(screen_h) {
        return Some(SplitterAxis::Inspector);
    }
    if !dock.bottom_collapsed && my >= bottom_y - hit && my <= bottom_y + hit && mx >= left_x && mx <= right_x {
        return Some(SplitterAxis::Bottom);
    }
    None
}

/// 根据拖拽位移更新停靠尺寸。
pub fn apply_splitter_drag(drag: SplitterDrag, mx: f32, my: f32, screen_w: f32, screen_h: f32, dock: &mut DockLayoutState) {
    match drag.axis {
        SplitterAxis::Hierarchy => {
            let delta = mx - drag.anchor;
            dock.hierarchy_width = dock.clamp_hierarchy(drag.start_hierarchy + delta, screen_w);
        }
        SplitterAxis::Inspector => {
            let delta = drag.anchor - mx;
            dock.inspector_width = dock.clamp_inspector(drag.start_inspector + delta, screen_w);
        }
        SplitterAxis::Bottom => {
            let delta = drag.anchor - my;
            dock.bottom_height = dock.clamp_bottom(drag.start_bottom + delta, screen_h);
        }
    }
}

/// 双击分栏：恢复该轴默认尺寸或切换折叠。
pub fn double_click_splitter(axis: SplitterAxis, dock: &mut DockLayoutState) {
    let default = DockLayoutState::default();
    match axis {
        SplitterAxis::Hierarchy => {
            if dock.hierarchy_width.abs() - default.hierarchy_width < 1.0 {
                dock.hierarchy_collapsed = !dock.hierarchy_collapsed;
            }
            else {
                dock.hierarchy_width = default.hierarchy_width;
                dock.hierarchy_collapsed = false;
            }
        }
        SplitterAxis::Inspector => {
            if dock.inspector_width.abs() - default.inspector_width < 1.0 {
                dock.inspector_collapsed = !dock.inspector_collapsed;
            }
            else {
                dock.inspector_width = default.inspector_width;
                dock.inspector_collapsed = false;
            }
        }
        SplitterAxis::Bottom => {
            if dock.bottom_height.abs() - default.bottom_height < 1.0 {
                dock.bottom_collapsed = !dock.bottom_collapsed;
            }
            else {
                dock.bottom_height = default.bottom_height;
                dock.bottom_collapsed = false;
            }
        }
    }
}
