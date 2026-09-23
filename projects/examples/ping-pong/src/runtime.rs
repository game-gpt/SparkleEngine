//! 乒乓：[`SparkRuntime`] 路径（拍/球 Component 实体 + 会话 Resource）。

use spark_ecs::{Entity, World};
use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime, SystemOrder};
use spark_input::Key;
use spark_types::{Color, Rect};

use crate::{ball::Ball, paddle::{Paddle, PaddleSide}};

const COURT_W: f32 = 960.0;
const COURT_H: f32 = 540.0;

/// 局级会话（比分、发球方向；**不含**拍/球坐标）。
#[derive(Debug)]
struct PingPongSession {
    score_l: u32,
    score_r: u32,
    exit: bool,
    serve_to_right: bool,
}

impl PingPongSession {
    fn new() -> Self {
        Self { score_l: 0, score_r: 0, exit: false, serve_to_right: true }
    }
}

fn spawn_match(world: &mut World, serve_to_right: bool) {
    world.spawn(Paddle::left(COURT_H));
    world.spawn(Paddle::right(COURT_W, COURT_H));
    world.spawn(Ball::serve(COURT_W, COURT_H, serve_to_right));
}

fn ball_entity(world: &World) -> Option<Entity> {
    let mut found = None;
    world.for_each::<Ball>(|entity, _| found = Some(entity));
    found
}

fn reset_ball(world: &mut World) {
    let to_right = {
        let session = world.resources.get_mut::<PingPongSession>().unwrap();
        let dir = session.serve_to_right;
        session.serve_to_right = !session.serve_to_right;
        dir
    };
    if let Some(entity) = ball_entity(world) {
        if let Some(ball) = world.get_mut::<Ball>(entity) {
            *ball = Ball::serve(COURT_W, COURT_H, to_right);
        }
    }
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

fn simulate_ball(world: &mut World) {
    let paddles: Vec<Paddle> = {
        let mut out = Vec::with_capacity(2);
        world.for_each::<Paddle>(|_, paddle| out.push(paddle.clone()));
        out
    };
    let Some(ball_entity) = ball_entity(world) else {
        return;
    };
    let Some(ball) = world.get_mut::<Ball>(ball_entity) else {
        return;
    };

    if ball.y - ball.radius < 0.0 {
        ball.y = ball.radius;
        ball.vy = ball.vy.abs();
    }
    else if ball.y + ball.radius > COURT_H {
        ball.y = COURT_H - ball.radius;
        ball.vy = -ball.vy.abs();
    }

    for paddle in &paddles {
        bounce_paddle(ball, paddle);
    }

    if ball.x + ball.radius < 0.0 {
        world.resources.get_mut::<PingPongSession>().unwrap().score_r += 1;
        reset_ball(world);
    }
    else if ball.x - ball.radius > COURT_W {
        world.resources.get_mut::<PingPongSession>().unwrap().score_l += 1;
        reset_ball(world);
    }
}

/// 乒乓 Rust 域插件。
pub struct PingPongNativePlugin;

impl NativeGamePlugin for PingPongNativePlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.scenes_mut().on_enter("play", |world| {
            if world.resources.get::<PingPongSession>().is_none() {
                world.resources.insert(PingPongSession::new());
                spawn_match(world, true);
            }
        });
        runtime.load_scene("play");

        runtime.add_system_ctx(RustPhase::Update, "ping_pong_input", |ctx| {
            if ctx.world.resources.get::<PingPongSession>().is_none() {
                return;
            }
            if ctx.input.key_pressed(Key::Escape) {
                ctx.world.resources.get_mut::<PingPongSession>().unwrap().exit = true;
                ctx.world.resources.get_mut::<spark_engine::AppExit>().unwrap().request();
                return;
            }

            let dt = ctx.dt;
            ctx.world.for_each_mut::<Paddle>(|_, paddle| {
                match paddle.side {
                    PaddleSide::Left => {
                        if ctx.input.key_down(Key::W) {
                            paddle.y -= paddle.speed * dt;
                        }
                        if ctx.input.key_down(Key::S) {
                            paddle.y += paddle.speed * dt;
                        }
                    }
                    PaddleSide::Right => {
                        if ctx.input.key_down(Key::Up) {
                            paddle.y -= paddle.speed * dt;
                        }
                        if ctx.input.key_down(Key::Down) {
                            paddle.y += paddle.speed * dt;
                        }
                    }
                }
                paddle.clamp_y(COURT_H);
            });
        });

        runtime.add_system_ctx_with_order(
            RustPhase::Update,
            "ping_pong_sim",
            SystemOrder { after: vec!["ping_pong_input"], ..SystemOrder::default() },
            |ctx| {
                let dt = ctx.dt;
                if let Some(entity) = ball_entity(ctx.world) {
                    if let Some(ball) = ctx.world.get_mut::<Ball>(entity) {
                        ball.x += ball.vx * dt;
                        ball.y += ball.vy * dt;
                    }
                }
                simulate_ball(ctx.world);
            },
        );

        runtime.add_render_fn("ping_pong_world", |world, _, draw| {
            draw.begin_world();
            draw.fill_rect(Rect::new(0.0, 0.0, COURT_W, COURT_H), Color::rgb(0.05, 0.07, 0.10));
            let mut y = 8.0_f32;
            while y < COURT_H {
                draw.fill_rect(Rect::new(COURT_W * 0.5 - 2.0, y, 4.0, 12.0), Color::rgba(1.0, 1.0, 1.0, 0.25));
                y += 22.0;
            }
            let paddle_c = Color::rgb(0.85, 0.9, 1.0);
            world.for_each::<Paddle>(|_, paddle| {
                draw.fill_rect(Rect::new(paddle.x, paddle.y, paddle.w, paddle.h), paddle_c);
            });
            world.for_each::<Ball>(|_, ball| {
                let d = ball.radius * 2.0;
                draw.fill_rect(
                    Rect::new(ball.x - ball.radius, ball.y - ball.radius, d, d),
                    Color::rgb(1.0, 0.85, 0.2),
                );
            });
        });

        runtime.add_system(RustPhase::UiPrepare, "ping_pong_hud", |world| {
            let session = world.resources.get::<PingPongSession>().unwrap();
            let mut batch = spark_renderer::UiRenderBatch::new();
            batch.text(24.0, 16.0, 28.0, Color::rgb(1.0, 1.0, 1.0), format!("{}   :   {}", session.score_l, session.score_r));
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
