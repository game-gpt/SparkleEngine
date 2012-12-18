//! 3D 游戏装配。与 [`crate::SparkApp`] 对称，不替代 2D 入口。

use spark_ecs::{Schedule, World};
use spark_renderer::DrawList3d;

use crate::{
    ecs_host::{AppExit, EcsHost3d},
    render3d::{RenderFrame3d, RenderSchedule3d, RenderSystem3d},
};

/// 3D 游戏装配根。建成后交给 [`EcsHost3d`]。
pub struct SparkApp3d {
    world: World,
    sim: Schedule,
    render: RenderSchedule3d,
}

impl Default for SparkApp3d {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkApp3d {
    pub fn new() -> Self {
        let mut world = World::new();
        world.resources.insert(AppExit::default());
        Self { world, sim: Schedule::new(), render: RenderSchedule3d::new() }
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.world.resources.insert(value);
        self
    }

    pub fn add_system(&mut self, name: &'static str, f: impl FnMut(&mut World) + Send + 'static) -> &mut Self {
        self.sim.add_fn(name, f);
        self
    }

    pub fn add_render_system(&mut self, system: impl RenderSystem3d + 'static) -> &mut Self {
        self.render.add_system(system);
        self
    }

    pub fn add_render_fn(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World, &RenderFrame3d, &mut DrawList3d) + Send + 'static,
    ) -> &mut Self {
        self.render.add_fn(name, f);
        self
    }

    pub fn add_plugin(&mut self, plugin: &dyn SparkPlugin3d) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn into_host(self) -> EcsHost3d {
        EcsHost3d::new(self.world, self.sim).with_renderer(self.render)
    }
}

/// 向 [`SparkApp3d`] 登记资源与系统。
pub trait SparkPlugin3d {
    fn build(&self, app: &mut SparkApp3d);
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Color;
    use spark_input::Input;
    use spark_renderer::{FrameCtx, GameHost3d, Mat4};

    struct Paint;

    impl SparkPlugin3d for Paint {
        fn build(&self, app: &mut SparkApp3d) {
            app.insert_resource(1u8);
            app.add_system("tick", |w| {
                *w.resources.get_mut::<u8>().unwrap() += 1;
            });
            app.add_render_fn("paint", |w, _, draw| {
                let n = *w.resources.get::<u8>().unwrap();
                draw.clear = Color::rgb(n as f32, 0.0, 0.0);
            });
        }
    }

    #[test]
    fn plugin_builds_3d_host() {
        let mut app = SparkApp3d::new();
        app.add_plugin(&Paint);
        let mut host = app.into_host();
        let input = Input::default();
        host.update(&FrameCtx { input: &input, dt: 0.016, screen_w: 800.0, screen_h: 600.0, timing: Default::default() });
        let mut draw = DrawList3d::new(Color::rgb(0.0, 0.0, 0.0), Mat4::IDENTITY);
        host.draw(&mut draw);
        assert!((draw.clear.r - 2.0).abs() < 1e-5);
    }
}
