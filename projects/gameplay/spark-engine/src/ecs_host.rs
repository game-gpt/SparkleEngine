//! ECS 与帧宿主桥：把 `Schedule` 挂到 `GameHost` / `GameHost3d`。
//!
//! 窗口泵与 GPU 提交仍在 `spark-renderer-wgpu`。本模块只接 ECS 与绘制相位。

use spark_ecs::{Schedule, World};
use spark_input::Input;
use spark_renderer::{Camera2d, DrawList, DrawList3d, FrameCtx, GameHost, GameHost3d};

use crate::{
    render2d::{RenderFrame2d, RenderSchedule2d},
    render3d::{RenderFrame3d, RenderSchedule3d},
};

/// 每帧写入 `World` 资源的帧快照（不含生命周期引用）。
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    pub dt: f32,
    /// 物理像素宽（与 `Input::mouse_pos` 同空间）。
    pub screen_w: f32,
    /// 物理像素高。
    pub screen_h: f32,
    /// 窗口 DPI 缩放。
    pub dpi_scale: f32,
    pub input: Input,
}

/// 进程退出请求（游戏系统写入，宿主在 `should_exit` 读取）。
#[derive(Debug, Default, Clone, Copy)]
pub struct AppExit {
    pub requested: bool,
}

impl AppExit {
    pub fn request(&mut self) {
        self.requested = true;
    }
}

/// 由游戏 / 渲染系统填充的 2D 绘制缓冲（系统写入，宿主在 draw 相位取走）。
#[derive(Debug, Default)]
pub struct DrawBuffer2d {
    pub list: Option<DrawList>,
}

/// 由游戏填充的 3D 绘制缓冲资源（系统写入，宿主在 draw 相位取走）。
#[derive(Debug, Default)]
pub struct DrawBuffer3d {
    pub list: Option<DrawList3d>,
}

fn insert_frame_snapshot(world: &mut World, frame: &FrameCtx<'_>) {
    let snap = FrameSnapshot {
        dt: frame.dt,
        screen_w: frame.screen_w,
        screen_h: frame.screen_h,
        dpi_scale: frame.dpi_scale,
        input: frame.input.clone(),
    };
    world.resources.insert(snap);
}

fn exit_requested(world: &World, host_exit: bool) -> bool {
    host_exit || world.resources.get::<AppExit>().is_some_and(|e| e.requested)
}

/// ECS 驱动的 2D 宿主。
///
/// `update`：写入 [`FrameSnapshot`] 后跑仿真 [`Schedule`]。  
/// `draw`：优先消费 [`DrawBuffer2d`]；否则跑 [`RenderSchedule2d`]；再否则旧 `draw_schedule` 或 `draw_fallback`。
pub struct EcsHost2d {
    pub world: World,
    pub schedule: Schedule,
    /// 正式 2D 绘制调度。
    pub renderer: RenderSchedule2d,
    /// 兼容路径：往 [`DrawBuffer2d`] 写整帧列表。
    pub draw_schedule: Schedule,
    pub exit: bool,
    draw_fallback: Option<Box<dyn FnMut(&mut World, &mut DrawList) + Send>>,
}

impl EcsHost2d {
    pub fn new(world: World, schedule: Schedule) -> Self {
        Self { world, schedule, renderer: RenderSchedule2d::new(), draw_schedule: Schedule::new(), exit: false, draw_fallback: None }
    }

    pub fn with_renderer(mut self, renderer: RenderSchedule2d) -> Self {
        self.renderer = renderer;
        self
    }

    pub fn with_draw_schedule(mut self, schedule: Schedule) -> Self {
        self.draw_schedule = schedule;
        self
    }

    pub fn with_draw_fallback(mut self, f: impl FnMut(&mut World, &mut DrawList) + Send + 'static) -> Self {
        self.draw_fallback = Some(Box::new(f));
        self
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn world(&self) -> &World {
        &self.world
    }
}

impl GameHost for EcsHost2d {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        insert_frame_snapshot(&mut self.world, frame);
        self.schedule.run(&mut self.world);
    }

    fn draw(&mut self, draw: &mut DrawList) {
        if let Some(buf) = self.world.resources.get_mut::<DrawBuffer2d>() {
            if let Some(list) = buf.list.take() {
                *draw = list;
                return;
            }
        }
        if let Some(cam) = self.world.resources.get::<Camera2d>().copied() {
            draw.set_camera(cam);
        }
        if !self.renderer.is_empty() {
            let (screen_w, screen_h) = self.world.resources.get::<FrameSnapshot>().map(|s| (s.screen_w, s.screen_h)).unwrap_or((0.0, 0.0));
            let frame = RenderFrame2d { screen_w, screen_h, clear: draw.clear };
            self.renderer.draw(&mut self.world, &frame, draw);
            return;
        }
        if !self.draw_schedule.is_empty() {
            self.world.resources.insert(DrawScratch2d { clear: draw.clear });
            self.draw_schedule.run(&mut self.world);
            if let Some(buf) = self.world.resources.get_mut::<DrawBuffer2d>() {
                if let Some(list) = buf.list.take() {
                    *draw = list;
                    return;
                }
            }
        }
        if let Some(fallback) = self.draw_fallback.as_mut() {
            fallback(&mut self.world, draw);
        }
    }

    fn should_exit(&self) -> bool {
        exit_requested(&self.world, self.exit)
    }
}

/// 绘制相位临时资源：供 `draw_schedule` 系统读取清屏色等。
#[derive(Debug, Clone, Copy)]
pub struct DrawScratch2d {
    pub clear: spark_types::Color,
}

/// ECS 驱动的 3D 宿主。
///
/// `update`：写入 [`FrameSnapshot`] 后跑 [`Schedule`]。  
/// `draw`：消费 [`DrawBuffer3d`]；否则跑 [`RenderSchedule3d`]；再否则 `draw_fallback`。
pub struct EcsHost3d {
    pub world: World,
    pub schedule: Schedule,
    pub renderer: RenderSchedule3d,
    pub exit: bool,
    pub grab_cursor: bool,
    draw_fallback: Option<Box<dyn FnMut(&World, &mut DrawList3d) + Send>>,
}

impl EcsHost3d {
    pub fn new(world: World, schedule: Schedule) -> Self {
        Self { world, schedule, renderer: RenderSchedule3d::new(), exit: false, grab_cursor: true, draw_fallback: None }
    }

    pub fn with_renderer(mut self, renderer: RenderSchedule3d) -> Self {
        self.renderer = renderer;
        self
    }

    pub fn with_draw_fallback(mut self, f: impl FnMut(&World, &mut DrawList3d) + Send + 'static) -> Self {
        self.draw_fallback = Some(Box::new(f));
        self
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn world(&self) -> &World {
        &self.world
    }
}

impl GameHost3d for EcsHost3d {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        insert_frame_snapshot(&mut self.world, frame);
        self.schedule.run(&mut self.world);
    }

    fn draw(&mut self, draw: &mut DrawList3d) {
        if let Some(buf) = self.world.resources.get_mut::<DrawBuffer3d>() {
            if let Some(list) = buf.list.take() {
                *draw = list;
                return;
            }
        }
        if !self.renderer.is_empty() {
            let (screen_w, screen_h) = self.world.resources.get::<FrameSnapshot>().map(|s| (s.screen_w, s.screen_h)).unwrap_or((0.0, 0.0));
            let frame = RenderFrame3d { screen_w, screen_h, clear: draw.clear };
            self.renderer.draw(&mut self.world, &frame, draw);
            return;
        }
        if let Some(fallback) = self.draw_fallback.as_mut() {
            fallback(&self.world, draw);
        }
    }

    fn should_exit(&self) -> bool {
        exit_requested(&self.world, self.exit)
    }

    fn cursor_grab(&self) -> bool {
        self.grab_cursor
    }
}
