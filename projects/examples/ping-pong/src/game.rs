//! 乒乓对局逻辑与绘制。

use spark_core::{Color, Rect};
use spark_input::Key;
use spark_renderer::{DrawList, FrameCtx, GameHost};

use crate::{ball::Ball, paddle::Paddle};

const COURT_W: f32 = 960.0;
const COURT_H: f32 = 540.0;

#[derive(Debug)]
pub struct PingPongGame {
    pub left: Paddle,
    pub right: Paddle,
    pub ball: Ball,
    score_l: u32,
    score_r: u32,
    exit: bool,
    serve_to_right: bool,
}

impl Default for PingPongGame {
    fn default() -> Self {
        Self::new()
    }
}

impl PingPongGame {
    pub fn new() -> Self {
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

impl GameHost for PingPongGame {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        if frame.input.key_pressed(Key::Escape) {
            self.exit = true;
            return;
        }

        let dt = frame.dt;
        if frame.input.key_down(Key::W) {
            self.left.y -= self.left.speed * dt;
        }
        if frame.input.key_down(Key::S) {
            self.left.y += self.left.speed * dt;
        }
        if frame.input.key_down(Key::Up) {
            self.right.y -= self.right.speed * dt;
        }
        if frame.input.key_down(Key::Down) {
            self.right.y += self.right.speed * dt;
        }
        self.left.clamp_y(COURT_H);
        self.right.clamp_y(COURT_H);

        self.ball.x += self.ball.vx * dt;
        self.ball.y += self.ball.vy * dt;

        if self.ball.y - self.ball.radius < 0.0 {
            self.ball.y = self.ball.radius;
            self.ball.vy = self.ball.vy.abs();
        }
        else if self.ball.y + self.ball.radius > COURT_H {
            self.ball.y = COURT_H - self.ball.radius;
            self.ball.vy = -self.ball.vy.abs();
        }

        Self::bounce_paddle(&mut self.ball, &self.left);
        Self::bounce_paddle(&mut self.ball, &self.right);

        if self.ball.x + self.ball.radius < 0.0 {
            self.score_r += 1;
            self.reset_ball();
        }
        else if self.ball.x - self.ball.radius > COURT_W {
            self.score_l += 1;
            self.reset_ball();
        }
    }

    fn draw(&mut self, draw: &mut DrawList) {
        draw.begin_world();
        draw.fill_rect(Rect::new(0.0, 0.0, COURT_W, COURT_H), Color::rgb(0.05, 0.07, 0.10));
        let mut y = 8.0_f32;
        while y < COURT_H {
            draw.fill_rect(Rect::new(COURT_W * 0.5 - 2.0, y, 4.0, 12.0), Color::rgba(1.0, 1.0, 1.0, 0.25));
            y += 22.0;
        }

        let paddle_c = Color::rgb(0.85, 0.9, 1.0);
        draw.fill_rect(Rect::new(self.left.x, self.left.y, self.left.w, self.left.h), paddle_c);
        draw.fill_rect(Rect::new(self.right.x, self.right.y, self.right.w, self.right.h), paddle_c);

        let d = self.ball.radius * 2.0;
        draw.fill_rect(Rect::new(self.ball.x - self.ball.radius, self.ball.y - self.ball.radius, d, d), Color::rgb(1.0, 0.85, 0.2));

        draw.begin_hud();
        draw.text(24.0, 16.0, 28.0, Color::rgb(1.0, 1.0, 1.0), format!("{}   :   {}", self.score_l, self.score_r));
        draw.text(24.0, COURT_H - 36.0, 16.0, Color::rgba(1.0, 1.0, 1.0, 0.55), "W/S 左拍 · ↑/↓ 右拍 · Esc 退出");
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
