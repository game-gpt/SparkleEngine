//! 贪吃蛇：[`SparkRuntime`] 路径（段实体 Component + 会话 Resource）。

use spark_ecs::{Entity, World};
use spark_engine::{NativeGamePlugin, RustPhase, SparkRuntime, SystemOrder};
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

/// 网格格点（每段实体一份）。
#[derive(Debug, Clone, Copy)]
struct GridPos {
    x: i32,
    y: i32,
}

/// 蛇身段序（0 = 头）。
#[derive(Debug, Clone, Copy)]
struct SnakeSegment {
    order: u32,
}

/// 局级会话状态（方向、食物、计时、胜负；**不含** `Vec` 段坐标）。
#[derive(Debug)]
struct SnakeSession {
    dir: Dir,
    pending: Option<Dir>,
    food: GridPos,
    acc: f32,
    step: f32,
    score: u32,
    dead: bool,
    exit: bool,
    rng: u64,
}

impl SnakeSession {
    fn new() -> Self {
        Self {
            dir: Dir::Right,
            pending: None,
            food: GridPos { x: 0, y: 0 },
            acc: 0.0,
            step: 0.12,
            score: 0,
            dead: false,
            exit: false,
            rng: 0x5A11E_u64,
        }
    }
}

struct SegmentSnap {
    entity: Entity,
    x: i32,
    y: i32,
    order: u32,
}

fn spawn_initial_snake(world: &mut World) {
    for (order, (x, y)) in [(8, 9), (7, 9), (6, 9)].into_iter().enumerate() {
        world.spawn2(GridPos { x, y }, SnakeSegment { order: order as u32 });
    }
}

fn collect_segments(world: &mut World) -> Vec<SegmentSnap> {
    let mut out = Vec::new();
    world.for_each2_mut::<GridPos, SnakeSegment>(|entity, pos, seg| {
        out.push(SegmentSnap { entity, x: pos.x, y: pos.y, order: seg.order });
    });
    out.sort_by_key(|s| s.order);
    out
}

fn despawn_all_segments(world: &mut World) {
    let entities: Vec<Entity> = collect_segments(world).into_iter().map(|s| s.entity).collect();
    for entity in entities {
        world.despawn(entity);
    }
}

fn occupied_cells(world: &World) -> std::collections::HashSet<(i32, i32)> {
    let mut blocked = std::collections::HashSet::new();
    world.for_each::<GridPos>(|_, pos| {
        blocked.insert((pos.x, pos.y));
    });
    blocked
}

fn place_food(world: &mut World) {
    let blocked = occupied_cells(&*world);
    let session = world.resources.get_mut::<SnakeSession>().unwrap();
    for _ in 0..512 {
        session.rng = session.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = ((session.rng >> 33) % COLS as u64) as i32;
        let y = ((session.rng >> 17) % ROWS as u64) as i32;
        if !blocked.contains(&(x, y)) {
            session.food = GridPos { x, y };
            return;
        }
    }
    session.food = GridPos { x: 0, y: 0 };
}

fn restart_session(world: &mut World) {
    despawn_all_segments(world);
    world.resources.insert(SnakeSession::new());
    spawn_initial_snake(world);
    place_food(world);
}

fn step_snake(world: &mut World) {
    {
        let session = world.resources.get_mut::<SnakeSession>().unwrap();
        if let Some(p) = session.pending.take() {
            if p != session.dir.opposite() {
                session.dir = p;
            }
        }
    }

    let segs = collect_segments(world);
    if segs.is_empty() {
        return;
    }

    let (dir, food_x, food_y) = {
        let session = world.resources.get::<SnakeSession>().unwrap();
        (session.dir, session.food.x, session.food.y)
    };

    let (dx, dy) = dir.delta();
    let next = (segs[0].x + dx, segs[0].y + dy);
    if next.0 < 0 || next.1 < 0 || next.0 >= COLS || next.1 >= ROWS {
        world.resources.get_mut::<SnakeSession>().unwrap().dead = true;
        return;
    }
    if segs.iter().any(|s| s.x == next.0 && s.y == next.1) {
        world.resources.get_mut::<SnakeSession>().unwrap().dead = true;
        return;
    }

    let ate = next.0 == food_x && next.1 == food_y;
    let old_positions: Vec<(i32, i32)> = segs.iter().map(|s| (s.x, s.y)).collect();

    if let Some(head_pos) = world.get_mut::<GridPos>(segs[0].entity) {
        head_pos.x = next.0;
        head_pos.y = next.1;
    }
    for seg in segs.iter().skip(1) {
        let (px, py) = old_positions[(seg.order - 1) as usize];
        if let Some(pos) = world.get_mut::<GridPos>(seg.entity) {
            pos.x = px;
            pos.y = py;
        }
    }

    if ate {
        let tail = old_positions[old_positions.len() - 1];
        let new_order = segs.last().map(|s| s.order + 1).unwrap_or(1);
        world.spawn2(GridPos { x: tail.0, y: tail.1 }, SnakeSegment { order: new_order });
        {
            let session = world.resources.get_mut::<SnakeSession>().unwrap();
            session.score += 1;
            session.step = (0.12 - session.score as f32 * 0.002).max(0.05);
        }
        place_food(world);
    }
    else if let Some(tail) = segs.last() {
        world.despawn(tail.entity);
    }
}

/// 贪吃蛇 Rust 域插件。
pub struct SnakeNativePlugin;

impl NativeGamePlugin for SnakeNativePlugin {
    fn build(&self, runtime: &mut SparkRuntime) {
        runtime.scenes_mut().on_enter("play", |world| {
            if world.resources.get::<SnakeSession>().is_none() {
                world.resources.insert(SnakeSession::new());
                spawn_initial_snake(world);
                place_food(world);
            }
        });
        runtime.load_scene("play");

        runtime.add_system_ctx(RustPhase::Update, "snake_input", |ctx| {
            let session = ctx.world.resources.get_mut::<SnakeSession>().unwrap();
            if ctx.input.key_pressed(Key::Escape) {
                session.exit = true;
                ctx.world.resources.get_mut::<spark_engine::AppExit>().unwrap().request();
                return;
            }
            if session.dead {
                if ctx.input.key_pressed(Key::R) {
                    restart_session(ctx.world);
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
                if d != session.dir.opposite() {
                    session.pending = Some(d);
                }
            }
        });

        runtime.add_system_ctx_with_order(
            RustPhase::Update,
            "snake_step",
            SystemOrder { after: vec!["snake_input"], ..SystemOrder::default() },
            |ctx| {
                if ctx.world.resources.get::<SnakeSession>().is_none_or(|s| s.dead) {
                    return;
                }
                {
                    let session = ctx.world.resources.get_mut::<SnakeSession>().unwrap();
                    session.acc += ctx.dt;
                }
                loop {
                    let step = ctx.world.resources.get::<SnakeSession>().unwrap().step;
                    let mut tick = false;
                    {
                        let session = ctx.world.resources.get_mut::<SnakeSession>().unwrap();
                        if session.acc >= step {
                            session.acc -= step;
                            tick = true;
                        }
                    }
                    if !tick {
                        break;
                    }
                    step_snake(ctx.world);
                    if ctx.world.resources.get::<SnakeSession>().is_none_or(|s| s.dead) {
                        break;
                    }
                }
            },
        );

        runtime.add_render_fn("snake_world", |world, _, draw| {
            let session = world.resources.get::<SnakeSession>().unwrap();
            let w = PAD * 2.0 + COLS as f32 * CELL;
            let h = PAD * 2.0 + ROWS as f32 * CELL + 40.0;
            draw.begin_world();
            draw.fill_rect(Rect::new(0.0, 0.0, w, h), Color::rgb(0.06, 0.08, 0.07));
            draw.fill_rect(
                Rect::new(PAD - 2.0, PAD - 2.0, COLS as f32 * CELL + 4.0, ROWS as f32 * CELL + 4.0),
                Color::rgb(0.1, 0.14, 0.12),
            );
            let (fx, fy) = (session.food.x, session.food.y);
            draw.fill_rect(
                Rect::new(PAD + fx as f32 * CELL + 2.0, PAD + fy as f32 * CELL + 2.0, CELL - 4.0, CELL - 4.0),
                Color::rgb(0.95, 0.35, 0.3),
            );
            world.for_each2_mut::<GridPos, SnakeSegment>(|_, pos, seg| {
                let c = if seg.order == 0 { Color::rgb(0.35, 0.95, 0.45) } else { Color::rgb(0.25, 0.7, 0.35) };
                draw.fill_rect(
                    Rect::new(PAD + pos.x as f32 * CELL + 1.0, PAD + pos.y as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0),
                    c,
                );
            });
        });

        runtime.add_system(RustPhase::UiPrepare, "snake_hud", |world| {
            let session = world.resources.get::<SnakeSession>().unwrap();
            let mut batch = spark_renderer::UiRenderBatch::new();
            batch.text(PAD, PAD + ROWS as f32 * CELL + 10.0, 20.0, Color::rgb(1.0, 1.0, 1.0), format!("Score {}", session.score));
            batch.text(
                PAD + 140.0,
                PAD + ROWS as f32 * CELL + 12.0,
                14.0,
                Color::rgba(1.0, 1.0, 1.0, 0.55),
                "方向键/WASD · R 重开 · Esc 退出",
            );
            if session.dead {
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
