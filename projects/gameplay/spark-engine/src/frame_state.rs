//! 帧资源与绘制缓冲：由仿真 / 渲染系统写入，窗口泵在 present 相位读取。

use spark_input::Input;
use spark_renderer::{DrawList, DrawList3d, UiRenderBatch};

/// 每帧写入 `World` 资源的帧快照（不含生命周期引用）。
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    /// 本仿真步 `dt`（秒；固定步时为固定值）。
    pub dt: f32,
    /// 物理像素宽（与 `Input::mouse_pos` 同空间）。
    pub screen_w: f32,
    /// 物理像素高。
    pub screen_h: f32,
    /// 窗口 DPI 缩放。
    pub dpi_scale: f32,
    /// 本帧输入快照（已 clone，可安全存入资源）。
    pub input: Input,
}

/// 进程退出请求（游戏系统写入，窗口泵在 `should_exit` 读取）。
#[derive(Debug, Default, Clone, Copy)]
pub struct AppExit {
    /// 为 true 时窗口泵应结束。
    pub requested: bool,
}

impl AppExit {
    /// 标记请求退出。
    pub fn request(&mut self) {
        self.requested = true;
    }
}

/// 操作系统光标可见性（游戏系统写入，2D 窗口泵每帧读取）。
///
/// 默认可见。标题菜单绘制自绘光标时应写入 `false`，避免双光标。
#[derive(Debug, Clone, Copy)]
pub struct OsCursorVisible(
    /// `true` = 显示 OS 光标。
    pub bool,
);

impl Default for OsCursorVisible {
    fn default() -> Self {
        Self(true)
    }
}

/// 由游戏 / 渲染系统填充的 2D 绘制缓冲（系统写入，窗口泵在 present 相位取走）。
#[derive(Debug, Default)]
pub struct DrawBuffer2d {
    /// 整帧 `DrawList`；窗口泵 `take` 后置 `None`。
    pub list: Option<DrawList>,
}

/// 由游戏 / UI 系统填充的 HUD 批次（系统写入，窗口泵在 `present_ui` 相位取走）。
#[derive(Debug, Default)]
pub struct UiBuffer2d {
    /// 本帧 UI 批次；窗口泵 `take` 后置 `None`。
    pub batch: Option<UiRenderBatch>,
}

/// 由游戏填充的 3D 绘制缓冲资源（系统写入，窗口泵在 present 相位取走）。
#[derive(Debug, Default)]
pub struct DrawBuffer3d {
    /// 整帧 `DrawList3d`；窗口泵 `take` 后置 `None`。
    pub list: Option<DrawList3d>,
}

/// 窗口泵指针抓取偏好（游戏系统每帧写入，3D 窗口泵在 `cursor_grab` 读取）。
#[derive(Debug, Default, Clone, Copy)]
pub struct CursorGrabPref(
    /// `true` = 请求指针锁定。
    pub bool,
);
