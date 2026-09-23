//! 乒乓：[`SparkRuntime`] 路径（权威状态在 ECS 资源）。

use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime};
use spark_input::Key;
use spark_types::{Color, Rect};

use crate::{ball::Ball, paddle::Paddle};

const COURT_W: f32 = 960.0;
const COURT_H: f32 = 540.0;

/// 乒乓局权威状态（内核 [`World`] 资源）。
#[derive(Debug)]
pub struct PingPongState {
    left: Paddle,
    right: Paddle,
    ball: Ball,
    score_l: u32,
    score_r: u32,
    exit: bool,
    serve_to_right: bool,
}

impl PingPongState {
    fn new() -> Self {
        Self {
            left: Paddle::left(COURT_H),
            right: Paddle::right(COURT_W, COURT_H),
            ball: Ball::serve(COURT_W, COURT_H, true),
            score_l: 0,
            score_r: 0,
            exit: false,
            serve_to_right: true,
        }
    }

    fn reset_ball(&mut self) {
        self.ball = Ball::serve(COURT_W, COURT_H, self.serve_to_right);
        self.serve_to_right = !self.serve_to_right;
    }

    fn bounce_paddle(ball: &mut Ball, paddle: &Paddle) {
        let cx = ball.x;
        let cy = ball.y;
        let r = ball.radius;
        let hit = cx + r >= paddle.x && cx - r <= paddle.x + paddle.w && cy + r >= paddle.y && cy - r <= paddle.y + paddle.h;
        if !hit {
            return;
        }
        let going_right = ball.vx > 0.0;
        let from_left = cx < paddle.x + paddle.w * 0.5;
        if going_right == from_left {
            return;
        }
        let rel = ((cy - paddle.y) / paddle.h).clamp(0.0, 1.0) - 0.5;
        ball.vx = -ball.vx;
        let speed = (ball.vx.hypot(ball.vy)).max(ball.speed);
        let dir_x = ball.vx.signum();
        ball.vx = dir_x * speed * 1.03;
        ball.vy = rel * speed * 1.6;
        if going_right {
            ball.x = paddle.x - r - 0.1;
        }
        else {
            ball.x = paddle.x + paddle.w + r + 0.1;
        }
    }
}

/// 乒乓 Rust 域插件。
pub struct PingPongNativePlugin;

impl NativeGamePlugin for PingPongNativePlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.insert_resource(PingPongState::new());
        runtime.scenes_mut().on_enter("play", |world| {
            if world.resources.get::<PingPongState>().is_none() {
                world.resources.insert(PingPongState::new());
            }
        });
        runtime.load_scene("play");

        runtime.add_system_ctx(RustPhase::Update, "ping_pong_sim", |ctx| {
            let state = ctx.world.resources.get_mut::<PingPongState>().unwrap();
            if ctx.input.key_pressed(Key::Escape) {
                state.exit = true;
                ctx.world.resources.get_mut::<spark_engine::AppExit>().unwrap().request();
                return;
            }

            let dt = ctx.dt;
            if ctx.input.key_down(Key::W) {
                state.left.y -= state.left.speed * dt;
            }
            if ctx.input.key_down(Key::S) {
                state.left.y += state.left.speed * dt;
            }
            if ctx.input.key_down(Key::Up) {
                state.right.y -= state.right.speed * dt;
            }
            if ctx.input.key_down(Key::Down) {
                state.right.y += state.right.speed * dt;
            }
            state.left.clamp_y(COURT_H);
            state.right.clamp_y(COURT_H);

            state.ball.x += state.ball.vx * dt;
            state.ball.y += state.ball.vy * dt;

            if state.ball.y - state.ball.radius < 0.0 {
                state.ball.y = state.ball.radius;
                state.ball.vy = state.ball.vy.abs();
            }
            else if state.ball.y + state.ball.radius > COURT_H {
                state.ball.y = COURT_H - state.ball.radius;
                state.ball.vy = -state.ball.vy.abs();
            }

            PingPongState::bounce_paddle(&mut state.ball, &state.left);
            PingPongState::bounce_paddle(&mut state.ball, &state.right);

            if state.ball.x + state.ball.radius < 0.0 {
                state.score_r += 1;
                state.reset_ball();
            }
            else if state.ball.x - state.ball.radius > COURT_W {
                state.score_l += 1;
                state.reset_ball();
            }
        });

        runtime.add_render_fn("ping_pong_world", |world, _, draw| {
            let state = world.resources.get::<PingPongState>().unwrap();
            draw.begin_world();
            draw.fill_rect(Rect::new(0.0, 0.0, COURT_W, COURT_H), Color::rgb(0.05, 0.07, 0.10));
            let mut y = 8.0_f32;
            while y < COURT_H {
                draw.fill_rect(Rect::new(COURT_W * 0.5 - 2.0, y, 4.0, 12.0), Color::rgba(1.0, 1.0, 1.0, 0.25));
                y += 22.0;
            }
            let paddle_c = Color::rgb(0.85, 0.9, 1.0);
            draw.fill_rect(Rect::new(state.left.x, state.left.y, state.left.w, state.left.h), paddle_c);
            draw.fill_rect(Rect::new(state.right.x, state.right.y, state.right.w, state.right.h), paddle_c);
            let d = state.ball.radius * 2.0;
            draw.fill_rect(
                Rect::new(state.ball.x - state.ball.radius, state.ball.y - state.ball.radius, d, d),
                Color::rgb(1.0, 0.85, 0.2),
            );
        });

        runtime.add_system(RustPhase::UiPrepare, "ping_pong_hud", |world| {
            let state = world.resources.get::<PingPongState>().unwrap();
            let mut batch = spark_renderer::UiRenderBatch::new();
            batch.text(24.0, 16.0, 28.0, Color::rgb(1.0, 1.0, 1.0), format!("{}   :   {}", state.score_l, state.score_r));
            batch.text(
                24.0,
                COURT_H - 36.0,
                16.0,
                Color::rgba(1.0, 1.0, 1.0, 0.55),
                "W/S 左拍 · ↑/↓ 右拍 · Esc 退出",
            );
            world.resources.get_mut::<spark_engine::UiBuffer2d>().unwrap().batch = Some(batch);
        });
    }
}

/// 装配并返回可 `run_runtime` 的运行时。
pub fn build_runtime() -> SparkRuntime {
    let mut runtime = SparkRuntime::new();
    runtime.register_native(&PingPongNativePlugin);
    runtime
}
