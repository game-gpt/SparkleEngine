//! Spark 渲染抽象层：绘制列表、帧上下文与宿主契约。
//!
//! **不含** GPU / 窗口后端。桌面 wgpu 实现见 `spark-renderer-wgpu`。
//! 游戏与 `spark-widget` 只依赖本 crate 的 `DrawList` / `GameHost` 等类型。

mod draw;

pub use draw::{DrawList, QuadCmd, TextCmd};
pub use spark_input::{ButtonState, Input, Key, MouseBtn};

/// 启动窗口配置（后端无关字段）。
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub clear_color: [f64; 4],
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Spark".into(),
            width: 1280,
            height: 720,
            clear_color: [0.05, 0.06, 0.10, 1.0],
        }
    }
}

/// 每帧输入与时间。
pub struct FrameCtx<'a> {
    pub input: &'a Input,
    pub dt: f32,
    pub screen_w: f32,
    pub screen_h: f32,
}

/// 游戏宿主：更新逻辑并填充绘制列表。
pub trait GameHost {
    fn update(&mut self, frame: &FrameCtx<'_>);
    fn draw(&mut self, draw: &mut DrawList);
    fn should_exit(&self) -> bool {
        false
    }
}
