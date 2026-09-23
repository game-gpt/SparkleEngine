//! 贪吃蛇：[`SparkRuntime`] 路径（权威状态在 ECS 资源）。

use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime};
use spark_input::Key;
use spark_types::{Color, Rect};

const COLS: i32 = 24;
const ROWS: i32 = 18;
const CELL: f32 = 28.0;
const PAD: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn delta(self) -> (i32, i32) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// 蛇局权威状态（内核 [`World`] 资源，非宿主 struct 字段）。
#[derive(Debug)]
pub struct SnakeState {
    body: Vec<(i32, i32)>,
    dir: Dir,
    pending: Option<Dir>,
    food: (i32, i32),
    acc: f32,
    step: f32,
    score: u32,
    dead: bool,
    exit: bool,
    rng: u64,
}

impl SnakeState {
    fn new() -> Self {
        let mut s = Self {
            body: vec![(8, 9), (7, 9), (6, 9)],
            dir: Dir::Right,
            pending: None,
            food: (0, 0),
            acc: 0.0,
            step: 0.12,
            score: 0,
            dead: false,
            exit: false,
            rng: 0x5A11E_u64,
        };
        s.place_food();
        s
    }

    fn place_food(&mut self) {
        for _ in 0..512 {
            self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let x = ((self.rng >> 33) % COLS as u64) as i32;
            let y = ((self.rng >> 17) % ROWS as u64) as i32;
            if !self.body.iter().any(|&p| p == (x, y)) {
                self.food = (x, y);
                return;
            }
        }
        self.food = (0, 0);
    }

    fn restart(&mut self) {
        *self = Self::new();
    }

    fn step_once(&mut self) {
        if let Some(p) = self.pending.take() {
            if p != self.dir.opposite() {
                self.dir = p;
            }
        }
        let (dx, dy) = self.dir.delta();
        let head = self.body[0];
        let next = (head.0 + dx, head.1 + dy);
        if next.0 < 0 || next.1 < 0 || next.0 >= COLS || next.1 >= ROWS {
            self.dead = true;
            return;
        }
        if self.body.contains(&next) {
            self.dead = true;
            return;
        }
        self.body.insert(0, next);
        if next == self.food {
            self.score += 1;
            self.step = (0.12 - self.score as f32 * 0.002).max(0.05);
            self.place_food();
        }
        else {
            self.body.pop();
        }
    }
}

/// 贪吃蛇 Rust 域插件。
pub struct SnakeNativePlugin;

impl NativeGamePlugin for SnakeNativePlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.insert_resource(SnakeState::new());
        runtime.scenes_mut().on_enter("play", |world| {
            if world.resources.get::<SnakeState>().is_none() {
                world.resources.insert(SnakeState::new());
            }
        });
        runtime.load_scene("play");

        runtime.add_system_ctx(RustPhase::Update, "snake_input_sim", |ctx| {
            let state = ctx.world.resources.get_mut::<SnakeState>().unwrap();
            if ctx.input.key_pressed(Key::Escape) {
                state.exit = true;
                ctx.world.resources.get_mut::<spark_engine::AppExit>().unwrap().request();
                return;
            }
            if state.dead {
                if ctx.input.key_pressed(Key::R) {
                    state.restart();
                }
                return;
            }
            let want = if ctx.input.key_pressed(Key::Up) || ctx.input.key_pressed(Key::W) {
                Some(Dir::Up)
            }
            else if ctx.input.key_pressed(Key::Down) || ctx.input.key_pressed(Key::S) {
                Some(Dir::Down)
            }
            else if ctx.input.key_pressed(Key::Left) || ctx.input.key_pressed(Key::A) {
                Some(Dir::Left)
            }
            else if ctx.input.key_pressed(Key::Right) || ctx.input.key_pressed(Key::D) {
                Some(Dir::Right)
            }
            else {
                None
            };
            if let Some(d) = want {
                if d != state.dir.opposite() {
                    state.pending = Some(d);
                }
            }
            state.acc += ctx.dt;
            while state.acc >= state.step {
                state.acc -= state.step;
                state.step_once();
                if state.dead {
                    break;
                }
            }
        });

        runtime.add_render_fn("snake_world", |world, _, draw| {
            let state = world.resources.get::<SnakeState>().unwrap();
            let w = PAD * 2.0 + COLS as f32 * CELL;
            let h = PAD * 2.0 + ROWS as f32 * CELL + 40.0;
            draw.begin_world();
            draw.fill_rect(Rect::new(0.0, 0.0, w, h), Color::rgb(0.06, 0.08, 0.07));
            draw.fill_rect(
                Rect::new(PAD - 2.0, PAD - 2.0, COLS as f32 * CELL + 4.0, ROWS as f32 * CELL + 4.0),
                Color::rgb(0.1, 0.14, 0.12),
            );
            let (fx, fy) = state.food;
            draw.fill_rect(
                Rect::new(PAD + fx as f32 * CELL + 2.0, PAD + fy as f32 * CELL + 2.0, CELL - 4.0, CELL - 4.0),
                Color::rgb(0.95, 0.35, 0.3),
            );
            for (i, &(x, y)) in state.body.iter().enumerate() {
                let c = if i == 0 { Color::rgb(0.35, 0.95, 0.45) } else { Color::rgb(0.25, 0.7, 0.35) };
                draw.fill_rect(Rect::new(PAD + x as f32 * CELL + 1.0, PAD + y as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0), c);
            }
        });

        runtime.add_system(RustPhase::UiPrepare, "snake_hud", |world| {
            let state = world.resources.get::<SnakeState>().unwrap();
            let mut batch = spark_renderer::UiRenderBatch::new();
            batch.text(PAD, PAD + ROWS as f32 * CELL + 10.0, 20.0, Color::rgb(1.0, 1.0, 1.0), format!("Score {}", state.score));
            batch.text(
                PAD + 140.0,
                PAD + ROWS as f32 * CELL + 12.0,
                14.0,
                Color::rgba(1.0, 1.0, 1.0, 0.55),
                "方向键/WASD · R 重开 · Esc 退出",
            );
            if state.dead {
                batch.fill_rect(Rect::new(PAD + 40.0, PAD + 160.0, 400.0, 80.0), Color::rgba(0.0, 0.0, 0.0, 0.7));
                batch.text(PAD + 140.0, PAD + 180.0, 28.0, Color::rgb(1.0, 0.45, 0.4), "GAME OVER");
                batch.text(PAD + 150.0, PAD + 215.0, 16.0, Color::rgb(1.0, 1.0, 1.0), "按 R 重新开始");
            }
            world.resources.get_mut::<spark_engine::UiBuffer2d>().unwrap().batch = Some(batch);
        });
    }
}

/// 装配并返回可 `run_runtime` 的运行时。
pub fn build_runtime() -> SparkRuntime {
    let mut runtime = SparkRuntime::new();
    runtime.register_native(&SnakeNativePlugin);
    runtime
}
