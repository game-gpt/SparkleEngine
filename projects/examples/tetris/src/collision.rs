//! 碰撞与落定。

use crate::{
    board::Board,
    pieces::{PieceKind, cells},
};

/// `(x, y)` 是否落在棋盘闭区间 `[0, width) × [0, height)`。
pub fn in_bounds(board: &Board, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as u32) < board.width && (y as u32) < board.height
}

/// 方块以 `(ox, oy)` 为原点、旋转 `rot` 时是否可放置。
///
/// `y < 0` 的格子允许（生成时顶部外溢），但横向仍须在板内且不得盖住非空格。
pub fn fits(board: &Board, kind: PieceKind, rot: u8, ox: i32, oy: i32) -> bool {
    for (cx, cy) in cells(kind, rot) {
        let x = ox + cx;
        let y = oy + cy;
        if y < 0 {
            if x < 0 || x as u32 >= board.width {
                return false;
            }
            continue;
        }
        if !in_bounds(board, x, y) || board.get(x, y) != 0 {
            return false;
        }
    }
    true
}

/// 将方块写入棋盘：可见格（`y >= 0`）写入 `kind.0`；顶部外溢格丢弃。
pub fn lock_piece(board: &mut Board, kind: PieceKind, rot: u8, ox: i32, oy: i32) {
    for (cx, cy) in cells(kind, rot) {
        let x = ox + cx;
        let y = oy + cy;
        if y >= 0 {
            board.set(x, y, kind.0);
        }
    }
}
