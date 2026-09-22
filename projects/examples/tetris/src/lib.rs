//! 混合示例：Rust 棋盘权威 + 可玩主循环（Valkyrie HUD 元数据仍在 assets/scripts）。

#![warn(missing_docs)]
mod board;
mod collision;
mod pieces;

pub use board::Board;
pub use pieces::PieceKind;

use spark_types::{Color, Rect};
use spark_input::Key;
use spark_renderer::{DrawList, FrameCtx, GameHost};

use crate::{
    collision::{fits, lock_piece},
    pieces::{cells, random_kind, rotate_cw},
};

const COLS: i32 = 10;
const ROWS: i32 = 20;
const CELL: f32 = 28.0;
const ORIGIN_X: f32 = 40.0;
const ORIGIN_Y: f32 = 40.0;

#[derive(Debug)]
struct Active {
    kind: PieceKind,
    rot: u8,
    x: i32,
    y: i32,
}

#[derive(Debug)]
pub struct TetrisApp {
    board: Board,
    active: Option<Active>,
    next: PieceKind,
    fall_acc: f32,
    fall_interval: f32,
    score: u32,
    lines: u32,
    game_over: bool,
    exit: bool,
    rng: u64,
}

impl Default for TetrisApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TetrisApp {
    pub fn new() -> Self {
        let mut rng = 0xC0FFEE_u64;
        let next = random_kind(&mut rng);
        let mut app = Self {
            board: Board::default(),
            active: None,
            next,
            fall_acc: 0.0,
            fall_interval: 0.55,
            score: 0,
            lines: 0,
            game_over: false,
            exit: false,
            rng,
        };
        app.spawn();
        app
    }

    fn spawn(&mut self) {
        let kind = self.next;
        self.next = random_kind(&mut self.rng);
        let active = Active { kind, rot: 0, x: 3, y: 0 };
        if !fits(&self.board, kind, active.rot, active.x, active.y) {
            self.game_over = true;
            self.active = None;
            return;
        }
        self.active = Some(active);
        self.fall_acc = 0.0;
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let Some(a) = self.active.as_ref()
        else {
            return false;
        };
        let nx = a.x + dx;
        let ny = a.y + dy;
        if fits(&self.board, a.kind, a.rot, nx, ny) {
            let a = self.active.as_mut().unwrap();
            a.x = nx;
            a.y = ny;
            true
        }
        else {
            false
        }
    }

    fn try_rotate(&mut self) {
        let Some(a) = self.active.as_ref()
        else {
            return;
        };
        let nrot = rotate_cw(a.rot);
        if fits(&self.board, a.kind, nrot, a.x, a.y) {
            self.active.as_mut().unwrap().rot = nrot;
            return;
        }
        // 简易墙踢
        for kick in [-1, 1, -2, 2] {
            if fits(&self.board, a.kind, nrot, a.x + kick, a.y) {
                let a = self.active.as_mut().unwrap();
                a.rot = nrot;
                a.x += kick;
                return;
            }
        }
    }

    fn hard_drop(&mut self) {
        while self.try_move(0, 1) {}
        self.lock_active();
    }

    fn lock_active(&mut self) {
        let Some(a) = self.active.take()
        else {
            return;
        };
        lock_piece(&mut self.board, a.kind, a.rot, a.x, a.y);
        let cleared = self.board.clear_lines();
        if cleared > 0 {
            self.lines += cleared;
            self.score += match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                _ => 800,
            };
            self.fall_interval = (0.55 - self.lines as f32 * 0.015).max(0.12);
        }
        self.spawn();
    }

    fn restart(&mut self) {
        *self = Self::new();
    }
}

impl GameHost for TetrisApp {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        if frame.input.key_pressed(Key::Escape) {
            self.exit = true;
            return;
        }
        if self.game_over {
            if frame.input.key_pressed(Key::R) {
                self.restart();
            }
            return;
        }

        if frame.input.key_pressed(Key::Left) {
            self.try_move(-1, 0);
        }
        if frame.input.key_pressed(Key::Right) {
            self.try_move(1, 0);
        }
        if frame.input.key_pressed(Key::Up) || frame.input.key_pressed(Key::X) {
            self.try_rotate();
        }
        if frame.input.key_pressed(Key::Space) {
            self.hard_drop();
            return;
        }

        let soft = frame.input.key_down(Key::Down);
        let interval = if soft { self.fall_interval * 0.12 } else { self.fall_interval };
        self.fall_acc += frame.dt;
        while self.fall_acc >= interval {
            self.fall_acc -= interval;
            if !self.try_move(0, 1) {
                self.lock_active();
                break;
            }
        }
    }

    fn draw(&mut self, draw: &mut DrawList) {
        draw.begin_world();
        draw.fill_rect(Rect::new(0.0, 0.0, 480.0, 720.0), Color::rgb(0.04, 0.05, 0.08));

        let board_w = COLS as f32 * CELL;
        let board_h = ROWS as f32 * CELL;
        draw.fill_rect(Rect::new(ORIGIN_X - 4.0, ORIGIN_Y - 4.0, board_w + 8.0, board_h + 8.0), Color::rgb(0.12, 0.14, 0.18));

        for y in 0..ROWS {
            for x in 0..COLS {
                let v = self.board.get(x, y);
                if v == 0 {
                    continue;
                }
                draw.fill_rect(
                    Rect::new(ORIGIN_X + x as f32 * CELL + 1.0, ORIGIN_Y + y as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0),
                    piece_color(PieceKind(v)),
                );
            }
        }

        if let Some(a) = &self.active {
            for (cx, cy) in cells(a.kind, a.rot) {
                let x = a.x + cx;
                let y = a.y + cy;
                if y < 0 {
                    continue;
                }
                draw.fill_rect(
                    Rect::new(ORIGIN_X + x as f32 * CELL + 1.0, ORIGIN_Y + y as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0),
                    piece_color(a.kind),
                );
            }
        }

        // Next preview
        let nx0 = ORIGIN_X + board_w + 28.0;
        let ny0 = ORIGIN_Y + 40.0;
        draw.text(nx0, ORIGIN_Y, 18.0, Color::rgb(0.8, 0.85, 1.0), "NEXT");
        for (cx, cy) in cells(self.next, 0) {
            draw.fill_rect(Rect::new(nx0 + cx as f32 * 22.0, ny0 + cy as f32 * 22.0, 20.0, 20.0), piece_color(self.next));
        }

        draw.begin_hud();
        draw.text(nx0, ny0 + 120.0, 20.0, Color::rgb(1.0, 1.0, 1.0), format!("Score {}", self.score));
        draw.text(nx0, ny0 + 150.0, 18.0, Color::rgb(0.85, 0.9, 1.0), format!("Lines {}", self.lines));
        draw.text(24.0, 680.0, 14.0, Color::rgba(1.0, 1.0, 1.0, 0.5), "←/→ 移动 · ↑/X 旋转 · ↓ 软降 · Space 硬降 · R 重开 · Esc 退出");
        if self.game_over {
            draw.fill_rect(Rect::new(60.0, 300.0, 360.0, 80.0), Color::rgba(0.0, 0.0, 0.0, 0.7));
            draw.text(120.0, 320.0, 28.0, Color::rgb(1.0, 0.4, 0.4), "GAME OVER");
            draw.text(130.0, 355.0, 16.0, Color::rgb(1.0, 1.0, 1.0), "按 R 重新开始");
        }
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}

fn piece_color(kind: PieceKind) -> Color {
    match kind.0 {
        1 => Color::rgb(0.2, 0.85, 0.9),   // I
        2 => Color::rgb(0.95, 0.85, 0.2),  // O
        3 => Color::rgb(0.7, 0.3, 0.9),    // T
        4 => Color::rgb(0.3, 0.85, 0.35),  // S
        5 => Color::rgb(0.9, 0.3, 0.3),    // Z
        6 => Color::rgb(0.25, 0.45, 0.95), // J
        7 => Color::rgb(0.95, 0.55, 0.15), // L
        _ => Color::rgb(0.5, 0.5, 0.5),
    }
}
