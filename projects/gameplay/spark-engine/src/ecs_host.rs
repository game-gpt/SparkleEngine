//! ECS 与 3D 宿主桥：把 `Schedule` 挂到 `GameHost3d` 帧相位上。

use spark_ecs::{Schedule, World};
use spark_input::Input;
use spark_renderer::{DrawList3d, FrameCtx, GameHost3d};

/// 每帧写入 `World` 资源的帧快照（不含生命周期引用）。
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    pub dt: f32,
    pub screen_w: f32,
    pub screen_h: f32,
    pub input: Input,
}

/// 由游戏填充的 3D 绘制缓冲资源（系统写入，宿主在 draw 相位取走）。
#[derive(Debug, Default)]
pub struct DrawBuffer3d {
    pub list: Option<DrawList3d>,
}

/// ECS 驱动的 3D 宿主。
///
/// `update`：写入 [`FrameSnapshot`] 后跑 [`Schedule`]。  
/// `draw`：消费 [`DrawBuffer3d`]；若无则调用 `draw_fallback`。
pub struct EcsHost3d {
    pub world: World,
    pub schedule: Schedule,
    pub exit: bool,
    pub grab_cursor: bool,
    draw_fallback: Option<Box<dyn FnMut(&World, &mut DrawList3d) + Send>>,
}

impl EcsHost3d {
    pub fn new(world: World, schedule: Schedule) -> Self {
        Self {
            world,
            schedule,
            exit: false,
            grab_cursor: true,
            draw_fallback: None,
        }
    }

    pub fn with_draw_fallback(
        mut self,
        f: impl FnMut(&World, &mut DrawList3d) + Send + 'static,
    ) -> Self {
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
        let snap = FrameSnapshot {
            dt: frame.dt,
            screen_w: frame.screen_w,
            screen_h: frame.screen_h,
            input: frame.input.clone(),
        };
        self.world.resources.insert(snap);
        self.schedule.run(&mut self.world);
    }

    fn draw(&mut self, draw: &mut DrawList3d) {
        if let Some(buf) = self.world.resources.get_mut::<DrawBuffer3d>() {
            if let Some(list) = buf.list.take() {
                *draw = list;
                return;
            }
        }
        if let Some(fallback) = self.draw_fallback.as_mut() {
            fallback(&self.world, draw);
        }
    }

    fn should_exit(&self) -> bool {
        self.exit
    }

    fn cursor_grab(&self) -> bool {
        self.grab_cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Color;
    use spark_ecs::{Schedule, World};
    use spark_input::Input;
    use spark_renderer::{Mat4, WindowConfig};

    #[test]
    fn schedule_fills_draw_buffer() {
        let mut world = World::new();
        world.resources.insert(DrawBuffer3d::default());
        let mut schedule = Schedule::new();
        schedule.add_fn("draw", |w| {
            let list = DrawList3d::new(Color::rgb(0.1, 0.2, 0.3), Mat4::IDENTITY);
            w.resources.get_mut::<DrawBuffer3d>().unwrap().list = Some(list);
        });
        let mut host = EcsHost3d::new(world, schedule);
        let input = Input::default();
        let frame = FrameCtx {
            input: &input,
            dt: 1.0 / 60.0,
            screen_w: 1280.0,
            screen_h: 720.0,
            timing: Default::default(),
        };
        host.update(&frame);
        let mut draw = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
        host.draw(&mut draw);
        assert!((draw.clear.r - 0.1).abs() < 1e-5);
        let _ = WindowConfig::default();
    }
}
