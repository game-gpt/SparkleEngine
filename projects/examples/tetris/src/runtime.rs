//! 俄罗斯方块：[`SparkRuntime`] 路径（活动方块 Component + 棋盘 Resource）。

use spark_ecs::{Entity, World};
use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime, SystemOrder};
use spark_input::Key;
use spark_types::{Color, Rect};

use crate::{
    board::Board,
    collision::{fits, lock_piece},
    pieces::{cells, random_kind, rotate_cw, PieceKind},
};

const COLS: i32 = 10;
const ROWS: i32 = 20;
const CELL: f32 = 28.0;
const ORIGIN_X: f32 = 40.0;
const ORIGIN_Y: f32 = 40.0;

/// 下落中的方块（单实体 Component）。
#[derive(Debug, Clone, Copy)]
struct ActivePiece {
    kind: PieceKind,
    rot: u8,
    x: i32,
    y: i32,
}

/// 局级会话（分数、下落计时、下一块预览；**不含**棋盘格与活动方块坐标）。
#[derive(Debug)]
struct TetrisSession {
    next: PieceKind,
    fall_acc: f32,
    fall_interval: f32,
    score: u32,
    lines: u32,
    game_over: bool,
    rng: u64,
}

fn active_entity(world: &World) -> Option<Entity> {
    let mut found = None;
    world.for_each::<ActivePiece>(|entity, _| found = Some(entity));
    found
}

fn despawn_active(world: &mut World) {
    if let Some(entity) = active_entity(world) {
        world.despawn(entity);
    }
}

fn init_match(world: &mut World) {
    let mut rng = 0xC0FFEE_u64;
    let next = random_kind(&mut rng);
    world.resources.insert(TetrisSession {
        next,
        fall_acc: 0.0,
        fall_interval: 0.55,
        score: 0,
        lines: 0,
        game_over: false,
        rng,
    });
    world.resources.insert(Board::default());
    spawn_piece(world);
}

fn spawn_piece(world: &mut World) {
    despawn_active(world);
    let kind = {
        let session = world.resources.get_mut::<TetrisSession>().unwrap();
        let kind = session.next;
        session.next = random_kind(&mut session.rng);
        kind
    };
    let candidate = ActivePiece { kind, rot: 0, x: 3, y: 0 };
    let board = world.resources.get::<Board>().unwrap();
    if !fits(board, candidate.kind, candidate.rot, candidate.x, candidate.y) {
        world.resources.get_mut::<TetrisSession>().unwrap().game_over = true;
        return;
    }
    world.spawn(candidate);
    world.resources.get_mut::<TetrisSession>().unwrap().fall_acc = 0.0;
}

fn try_move(world: &mut World, dx: i32, dy: i32) -> bool {
    let Some(entity) = active_entity(world) else {
        return false;
    };
    let piece = *world.get::<ActivePiece>(entity).unwrap();
    let board = world.resources.get::<Board>().unwrap();
    let nx = piece.x + dx;
    let ny = piece.y + dy;
    if !fits(board, piece.kind, piece.rot, nx, ny) {
        return false;
    }
    let active = world.get_mut::<ActivePiece>(entity).unwrap();
    active.x = nx;
    active.y = ny;
    true
}

fn try_rotate(world: &mut World) {
    let Some(entity) = active_entity(world) else {
        return;
    };
    let piece = *world.get::<ActivePiece>(entity).unwrap();
    let nrot = rotate_cw(piece.rot);
    let board = world.resources.get::<Board>().unwrap();
    if fits(board, piece.kind, nrot, piece.x, piece.y) {
        let active = world.get_mut::<ActivePiece>(entity).unwrap();
        active.rot = nrot;
        return;
    }
    for kick in [-1, 1, -2, 2] {
        if fits(board, piece.kind, nrot, piece.x + kick, piece.y) {
            let active = world.get_mut::<ActivePiece>(entity).unwrap();
            active.rot = nrot;
            active.x += kick;
            return;
        }
    }
}

fn lock_active(world: &mut World) {
    let Some(entity) = active_entity(world) else {
        return;
    };
    let piece = *world.get::<ActivePiece>(entity).unwrap();
    world.despawn(entity);
    let board = world.resources.get_mut::<Board>().unwrap();
    lock_piece(board, piece.kind, piece.rot, piece.x, piece.y);
    let cleared = board.clear_lines();
    if cleared > 0 {
        let session = world.resources.get_mut::<TetrisSession>().unwrap();
        session.lines += cleared;
        session.score += match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            _ => 800,
        };
        session.fall_interval = (0.55 - session.lines as f32 * 0.015).max(0.12);
    }
    spawn_piece(world);
}

fn hard_drop(world: &mut World) {
    while try_move(world, 0, 1) {}
    lock_active(world);
}

fn restart_match(world: &mut World) {
    despawn_active(world);
    init_match(world);
}

fn piece_color(kind: PieceKind) -> Color {
    match kind.0 {
        1 => Color::rgb(0.2, 0.85, 0.9),
        2 => Color::rgb(0.95, 0.85, 0.2),
        3 => Color::rgb(0.7, 0.3, 0.9),
        4 => Color::rgb(0.3, 0.85, 0.35),
        5 => Color::rgb(0.9, 0.3, 0.3),
        6 => Color::rgb(0.25, 0.45, 0.95),
        7 => Color::rgb(0.95, 0.55, 0.15),
        _ => Color::rgb(0.5, 0.5, 0.5),
    }
}

/// 俄罗斯方块 Rust 域插件。
pub struct TetrisNativePlugin;

impl NativeGamePlugin for TetrisNativePlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.scenes_mut().on_enter("play", |world| {
            if world.resources.get::<TetrisSession>().is_none() {
                init_match(world);
            }
        });
        runtime.load_scene("play");

        runtime.add_system_ctx(RustPhase::Update, "tetris_input", |ctx| {
            if ctx.world.resources.get::<TetrisSession>().is_none() {
                return;
            }
            if ctx.input.key_pressed(Key::Escape) {
                ctx.world.resources.get_mut::<spark_engine::AppExit>().unwrap().request();
                return;
            }
            let game_over = ctx.world.resources.get::<TetrisSession>().unwrap().game_over;
            if game_over {
                if ctx.input.key_pressed(Key::R) {
                    restart_match(ctx.world);
                }
                return;
            }

            if ctx.input.key_pressed(Key::Left) {
                try_move(ctx.world, -1, 0);
            }
            if ctx.input.key_pressed(Key::Right) {
                try_move(ctx.world, 1, 0);
            }
            if ctx.input.key_pressed(Key::Up) || ctx.input.key_pressed(Key::X) {
                try_rotate(ctx.world);
            }
            if ctx.input.key_pressed(Key::Space) {
                hard_drop(ctx.world);
            }
        });

        runtime.add_system_ctx_with_order(
            RustPhase::Update,
            "tetris_gravity",
            SystemOrder { after: vec!["tetris_input"], ..SystemOrder::default() },
            |ctx| {
                if ctx.world.resources.get::<TetrisSession>().is_none_or(|s| s.game_over) {
                    return;
                }
                let soft = ctx.input.key_down(Key::Down);
                let interval = {
                    let session = ctx.world.resources.get::<TetrisSession>().unwrap();
                    if soft { session.fall_interval * 0.12 } else { session.fall_interval }
                };
                let mut tick = false;
                {
                    let session = ctx.world.resources.get_mut::<TetrisSession>().unwrap();
                    session.fall_acc += ctx.dt;
                    if session.fall_acc >= interval {
                        session.fall_acc -= interval;
                        tick = true;
                    }
                }
                if tick && !try_move(ctx.world, 0, 1) {
                    lock_active(ctx.world);
                }
            },
        );

        runtime.add_render_fn("tetris_world", |world, _, draw| {
            let board = world.resources.get::<Board>().unwrap();
            let session = world.resources.get::<TetrisSession>().unwrap();
            draw.begin_world();
            draw.fill_rect(Rect::new(0.0, 0.0, 480.0, 720.0), Color::rgb(0.04, 0.05, 0.08));

            let board_w = COLS as f32 * CELL;
            let board_h = ROWS as f32 * CELL;
            draw.fill_rect(Rect::new(ORIGIN_X - 4.0, ORIGIN_Y - 4.0, board_w + 8.0, board_h + 8.0), Color::rgb(0.12, 0.14, 0.18));

            for y in 0..ROWS {
                for x in 0..COLS {
                    let v = board.get(x, y);
                    if v == 0 {
                        continue;
                    }
                    draw.fill_rect(
                        Rect::new(ORIGIN_X + x as f32 * CELL + 1.0, ORIGIN_Y + y as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0),
                        piece_color(PieceKind(v)),
                    );
                }
            }

            world.for_each::<ActivePiece>(|_, active| {
                for (cx, cy) in cells(active.kind, active.rot) {
                    let x = active.x + cx;
                    let y = active.y + cy;
                    if y < 0 {
                        continue;
                    }
                    draw.fill_rect(
                        Rect::new(ORIGIN_X + x as f32 * CELL + 1.0, ORIGIN_Y + y as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0),
                        piece_color(active.kind),
                    );
                }
            });

            let nx0 = ORIGIN_X + board_w + 28.0;
            let ny0 = ORIGIN_Y + 40.0;
            draw.text(nx0, ORIGIN_Y, 18.0, Color::rgb(0.8, 0.85, 1.0), "NEXT");
            for (cx, cy) in cells(session.next, 0) {
                draw.fill_rect(Rect::new(nx0 + cx as f32 * 22.0, ny0 + cy as f32 * 22.0, 20.0, 20.0), piece_color(session.next));
            }
        });

        runtime.add_system(RustPhase::UiPrepare, "tetris_hud", |world| {
            let session = world.resources.get::<TetrisSession>().unwrap();
            let board_w = COLS as f32 * CELL;
            let nx0 = ORIGIN_X + board_w + 28.0;
            let ny0 = ORIGIN_Y + 40.0;
            let mut batch = spark_renderer::UiRenderBatch::new();
            batch.text(nx0, ny0 + 120.0, 20.0, Color::rgb(1.0, 1.0, 1.0), format!("Score {}", session.score));
            batch.text(nx0, ny0 + 150.0, 18.0, Color::rgb(0.85, 0.9, 1.0), format!("Lines {}", session.lines));
            batch.text(
                24.0,
                680.0,
                14.0,
                Color::rgba(1.0, 1.0, 1.0, 0.5),
                "←/→ 移动 · ↑/X 旋转 · ↓ 软降 · Space 硬降 · R 重开 · Esc 退出",
            );
            if session.game_over {
                batch.fill_rect(Rect::new(60.0, 300.0, 360.0, 80.0), Color::rgba(0.0, 0.0, 0.0, 0.7));
                batch.text(120.0, 320.0, 28.0, Color::rgb(1.0, 0.4, 0.4), "GAME OVER");
                batch.text(130.0, 355.0, 16.0, Color::rgb(1.0, 1.0, 1.0), "按 R 重新开始");
            }
            world.resources.get_mut::<spark_engine::UiBuffer2d>().unwrap().batch = Some(batch);
        });
    }
}

/// 装配并返回可 `run_runtime` 的运行时。
pub fn build_runtime() -> SparkRuntime {
    let mut runtime = SparkRuntime::new();
    runtime.register_native(&TetrisNativePlugin);
    runtime
}
