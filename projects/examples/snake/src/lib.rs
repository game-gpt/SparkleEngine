//! 贪吃蛇：Valkyrie 项目的 native 试玩宿主（脚本仍为 Inspector 元数据，VM Play 接通前由此运行）。

#![warn(missing_docs)]
use spark_core::{Color, Rect};
use spark_input::Key;
use spark_renderer::{DrawList, FrameCtx, GameHost};

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

#[derive(Debug)]
pub struct SnakeApp {
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

impl Default for SnakeApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SnakeApp {
    pub fn new() -> Self {
        let rng = 0x5A11E_u64;
        let mut app = Self {
            body: vec![(8, 9), (7, 9), (6, 9)],
            dir: Dir::Right,
            pending: None,
            food: (0, 0),
            acc: 0.0,
            step: 0.12,
            score: 0,
            dead: false,
            exit: false,
            rng,
        };
        app.place_food();
        app
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

impl GameHost for SnakeApp {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        if frame.input.key_pressed(Key::Escape) {
            self.exit = true;
            return;
        }
        if self.dead {
            if frame.input.key_pressed(Key::R) {
                self.restart();
            }
            return;
        }

        let want = if frame.input.key_pressed(Key::Up) || frame.input.key_pressed(Key::W) {
            Some(Dir::Up)
        }
        else if frame.input.key_pressed(Key::Down) || frame.input.key_pressed(Key::S) {
            Some(Dir::Down)
        }
        else if frame.input.key_pressed(Key::Left) || frame.input.key_pressed(Key::A) {
            Some(Dir::Left)
        }
        else if frame.input.key_pressed(Key::Right) || frame.input.key_pressed(Key::D) {
            Some(Dir::Right)
        }
        else {
            None
        };
        if let Some(d) = want {
            if d != self.dir.opposite() {
                self.pending = Some(d);
            }
        }

        self.acc += frame.dt;
        while self.acc >= self.step {
            self.acc -= self.step;
            self.step_once();
            if self.dead {
                break;
            }
        }
    }

    fn draw(&mut self, draw: &mut DrawList) {
        let w = PAD * 2.0 + COLS as f32 * CELL;
        let h = PAD * 2.0 + ROWS as f32 * CELL + 40.0;
        draw.begin_world();
        draw.fill_rect(Rect::new(0.0, 0.0, w, h), Color::rgb(0.06, 0.08, 0.07));
        draw.fill_rect(Rect::new(PAD - 2.0, PAD - 2.0, COLS as f32 * CELL + 4.0, ROWS as f32 * CELL + 4.0), Color::rgb(0.1, 0.14, 0.12));

        let (fx, fy) = self.food;
        draw.fill_rect(
            Rect::new(PAD + fx as f32 * CELL + 2.0, PAD + fy as f32 * CELL + 2.0, CELL - 4.0, CELL - 4.0),
            Color::rgb(0.95, 0.35, 0.3),
        );

        for (i, &(x, y)) in self.body.iter().enumerate() {
            let c = if i == 0 { Color::rgb(0.35, 0.95, 0.45) } else { Color::rgb(0.25, 0.7, 0.35) };
            draw.fill_rect(Rect::new(PAD + x as f32 * CELL + 1.0, PAD + y as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0), c);
        }

        draw.begin_hud();
        draw.text(PAD, PAD + ROWS as f32 * CELL + 10.0, 20.0, Color::rgb(1.0, 1.0, 1.0), format!("Score {}", self.score));
        draw.text(PAD + 140.0, PAD + ROWS as f32 * CELL + 12.0, 14.0, Color::rgba(1.0, 1.0, 1.0, 0.55), "方向键/WASD · R 重开 · Esc 退出");
        if self.dead {
            draw.fill_rect(Rect::new(PAD + 40.0, PAD + 160.0, 400.0, 80.0), Color::rgba(0.0, 0.0, 0.0, 0.7));
            draw.text(PAD + 140.0, PAD + 180.0, 28.0, Color::rgb(1.0, 0.45, 0.4), "GAME OVER");
            draw.text(PAD + 150.0, PAD + 215.0, 16.0, Color::rgb(1.0, 1.0, 1.0), "按 R 重新开始");
        }
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
